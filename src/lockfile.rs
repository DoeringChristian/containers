//! Lockfile management for container rebuilding
//!
//! This module provides functionality to track Dockerfile changes and automatically
//! trigger container rebuilds when the Dockerfile content has been modified.
//! The lockfile stores metadata about the current Dockerfile state including
//! file hash, modification time, and path information.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Lockfile structure that tracks Dockerfile states
///
/// The lockfile contains metadata about Dockerfiles to detect changes
/// and trigger rebuilds when necessary. It stores information about
/// file hashes, modification times, and paths.
#[derive(Debug, Serialize, Deserialize)]
pub struct Lockfile {
    /// Version of the lockfile format
    pub version: u32,
    /// Map of dockerfile paths to their metadata
    pub dockerfiles: HashMap<PathBuf, DockerfileInfo>,
}

/// Metadata about a specific Dockerfile
#[derive(Debug, Serialize, Deserialize)]
pub struct DockerfileInfo {
    /// SHA-256 hash of the Dockerfile content
    pub content_hash: String,
    /// Last modification time as Unix timestamp
    pub modified_time: u64,
    /// Size of the Dockerfile in bytes
    pub size: u64,
}

impl Lockfile {
    /// Current version of the lockfile format
    const VERSION: u32 = 1;

    /// Default lockfile name
    const LOCKFILE_NAME: &'static str = ".containers.lock";

    /// Creates a new empty lockfile
    pub fn new() -> Self {
        Self {
            version: Self::VERSION,
            dockerfiles: HashMap::new(),
        }
    }

    /// Loads an existing lockfile from disk, or creates a new one if it doesn't exist
    ///
    /// The lockfile is stored in the same directory as the Dockerfile being tracked.
    /// If multiple Dockerfiles are in different directories, each will have its own lockfile.
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Path to the Dockerfile to track
    ///
    /// # Returns
    ///
    /// Returns a `Result<Lockfile>` with the loaded or new lockfile, or an error if loading fails.
    pub fn load_or_create(dockerfile_path: &Path) -> Result<Self> {
        let lockfile_path = Self::get_lockfile_path(dockerfile_path)?;

        if lockfile_path.exists() {
            let content = fs::read_to_string(&lockfile_path)
                .with_context(|| format!("Failed to read lockfile: {}", lockfile_path.display()))?;

            let lockfile: Lockfile = serde_json::from_str(&content).with_context(|| {
                format!("Failed to parse lockfile: {}", lockfile_path.display())
            })?;

            Ok(lockfile)
        } else {
            Ok(Self::new())
        }
    }

    /// Saves the lockfile to disk
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Path to the Dockerfile this lockfile tracks
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or an error if saving fails.
    pub fn save(&self, dockerfile_path: &Path) -> Result<()> {
        let lockfile_path = Self::get_lockfile_path(dockerfile_path)?;

        let content = serde_json::to_string_pretty(self).context("Failed to serialize lockfile")?;

        fs::write(&lockfile_path, content)
            .with_context(|| format!("Failed to write lockfile: {}", lockfile_path.display()))?;

        Ok(())
    }

    /// Updates the lockfile with current Dockerfile information
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Path to the Dockerfile to update information for
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or an error if updating fails.
    pub fn update_dockerfile_info(&mut self, dockerfile_path: &Path) -> Result<()> {
        let info = DockerfileInfo::from_path(dockerfile_path)?;
        self.dockerfiles.insert(dockerfile_path.to_path_buf(), info);
        Ok(())
    }

    /// Checks if a Dockerfile has changed since the last lockfile update
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Path to the Dockerfile to check
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the Dockerfile has changed, `Ok(false)` if unchanged,
    /// or an error if the check fails.
    pub fn has_dockerfile_changed(&self, dockerfile_path: &Path) -> Result<bool> {
        let current_info = DockerfileInfo::from_path(dockerfile_path)?;

        match self.dockerfiles.get(dockerfile_path) {
            Some(stored_info) => Ok(stored_info.content_hash != current_info.content_hash
                || stored_info.modified_time != current_info.modified_time
                || stored_info.size != current_info.size),
            None => Ok(true), // No previous record means it's changed (new)
        }
    }

    /// Gets the path where the lockfile should be stored
    ///
    /// The lockfile is stored in the same directory as the Dockerfile.
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Path to the Dockerfile
    ///
    /// # Returns
    ///
    /// Returns the path where the lockfile should be located.
    fn get_lockfile_path(dockerfile_path: &Path) -> Result<PathBuf> {
        let dockerfile_dir = dockerfile_path
            .parent()
            .context("Dockerfile has no parent directory")?;

        Ok(dockerfile_dir.join(Self::LOCKFILE_NAME))
    }
}

impl DockerfileInfo {
    /// Creates DockerfileInfo from a file path by reading file metadata and content
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Dockerfile
    ///
    /// # Returns
    ///
    /// Returns a `Result<DockerfileInfo>` with the file information or an error.
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = fs::read(path)
            .with_context(|| format!("Failed to read Dockerfile: {}", path.display()))?;

        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get Dockerfile metadata: {}", path.display()))?;

        let modified_time = metadata
            .modified()
            .context("Failed to get modification time")?
            .duration_since(SystemTime::UNIX_EPOCH)
            .context("Invalid modification time")?
            .as_secs();

        let content_hash = Self::calculate_hash(&content);

        Ok(Self {
            content_hash,
            modified_time,
            size: content.len() as u64,
        })
    }

    /// Calculates SHA-256 hash of content
    fn calculate_hash(content: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}
