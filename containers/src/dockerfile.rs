//! Dockerfile location utilities
//!
//! This module provides functionality to automatically locate Dockerfiles
//! by searching upward from the current directory through the filesystem
//! hierarchy until reaching the home directory or filesystem root.

use std::env;
use std::path::{Path, PathBuf};

/// Utility for locating Dockerfiles in the filesystem
///
/// Provides methods to automatically discover Dockerfiles by searching
/// upward through the directory tree from the current working directory.
pub struct DockerfileLocator;

impl DockerfileLocator {
    /// Searches for a Dockerfile starting from the current directory
    ///
    /// This method implements a search strategy that:
    /// 1. Starts from the current working directory
    /// 2. Searches upward through parent directories
    /// 3. Stops at the user's home directory or filesystem root
    /// 4. Checks the home directory as a final fallback
    ///
    /// # Returns
    ///
    /// Returns `Some(PathBuf)` with the path to the first Dockerfile found,
    /// or `None` if no Dockerfile is found in the search path.
    ///
    /// # Search Order
    ///
    /// 1. Current working directory and all parents up to home directory
    /// 2. Home directory as final check
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use dockerfile::DockerfileLocator;
    ///
    /// if let Some(dockerfile_path) = DockerfileLocator::find() {
    ///     println!("Found Dockerfile at: {}", dockerfile_path.display());
    /// } else {
    ///     println!("No Dockerfile found");
    /// }
    /// ```
    pub fn find() -> Option<PathBuf> {
        let mut dir = env::current_dir().ok()?;
        let home_dir = home::home_dir()?;

        loop {
            let dockerfile = dir.join("Dockerfile");
            if dockerfile.exists() {
                return Some(dockerfile);
            }

            if dir == home_dir {
                break;
            }

            if dir == Path::new("/") {
                break;
            }

            dir = dir.parent()?.to_path_buf();
        }

        // Check home directory
        let home_dockerfile = home_dir.join("Dockerfile");
        if home_dockerfile.exists() {
            return Some(home_dockerfile);
        }

        None
    }
}
