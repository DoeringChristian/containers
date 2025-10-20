//! Configuration management for the container utility
//!
//! This module handles parsing command-line arguments, environment variables,
//! and creating a unified configuration structure for the application.

use anyhow::Result;
use std::env;
use std::path::PathBuf;

use crate::Args;
use crate::dockerfile::DockerfileLocator;
use crate::engine::EngineType;
use crate::lockfile::{DockerfileInfo, Lockfile};

/// Application configuration structure
///
/// Contains all settings needed to run containers, including paths,
/// names, and behavioral flags. Configuration is built from command-line
/// arguments and environment variables.
#[derive(Debug)]
pub struct Config {
    /// Path to the Dockerfile to use for building the container image
    pub dockerfile: PathBuf,
    /// Name of the container to create or connect to
    pub container_name: String,
    /// Name of the container image to build or use
    pub image_name: String,
    /// Container engine type (docker or podman)
    pub engine_type: EngineType,
    /// Whether to force rebuild the image and recreate the container
    pub update_image: bool,
    /// Custom command to run in the container (empty means use default shell)
    pub custom_command: Vec<String>,
    /// Lockfile for tracking Dockerfile state
    pub lockfile: Lockfile,
    /// Host user's UID for container user mapping
    pub user_uid: u32,
    /// Host user's GID for container user mapping
    pub user_gid: u32,
}

impl Config {
    /// Creates a new configuration from command-line arguments and environment variables
    ///
    /// This method combines CLI arguments with environment variable defaults to create
    /// a complete configuration. It handles:
    /// - Dockerfile location detection (CLI arg > env var > automatic search > fallback)
    /// - Container name generation based on Dockerfile location
    /// - Image name generation based on Dockerfile location
    /// - Container engine selection (env var or default to podman)
    ///
    /// # Arguments
    ///
    /// * `args` - Parsed command-line arguments
    ///
    /// # Returns
    ///
    /// Returns a `Result<Config>` with the complete configuration or an error.
    ///
    /// # Environment Variables
    ///
    /// * `CONTAINER_ENGINE` - Container engine to use (docker/podman, defaults to podman)
    /// * `DOCKERFILE` - Path to Dockerfile (overridden by CLI arg)
    /// * `CONTAINER_NAME` - Container name (overridden by CLI arg)
    pub fn from_args_and_env(args: Args) -> Result<Self> {
        let engine_type = env::var("CONTAINER_ENGINE")
            .unwrap_or_else(|_| "podman".to_string())
            .parse::<EngineType>()
            .unwrap_or_default();

        // Find Dockerfile
        let dockerfile = if let Some(dockerfile) = args.dockerfile {
            dockerfile
        } else if let Ok(dockerfile) = env::var("DOCKERFILE") {
            PathBuf::from(dockerfile)
        } else {
            DockerfileLocator::find().ok_or_else(|| {
                anyhow::anyhow!(
                    "No Dockerfile found. Searched from current directory up to home directory.\n\
                     You can specify a Dockerfile with:\n\
                     - The -f/--dockerfile flag\n\
                     - The DOCKERFILE environment variable\n\
                     - Or create a Dockerfile in the current directory or any parent directory"
                )
            })?
        };

        // Load or create lockfile to get the content hash
        let lockfile = Lockfile::load_or_create(&dockerfile)?;

        // Get or calculate the Dockerfile content hash
        let dockerfile_info = DockerfileInfo::from_path(&dockerfile)?;
        let content_hash = &dockerfile_info.content_hash;

        // Use first 12 characters of hash for container/image naming (like Docker short IDs)
        let hash_prefix = &content_hash[..12];

        // Set container name
        let default_container_name = hash_prefix.to_string();
        let container_name = if let Some(name) = args.container_name {
            name
        } else {
            env::var("CONTAINER_NAME").unwrap_or(default_container_name)
        };

        // Generate image name based on hash
        let image_name = format!("{}:latest", hash_prefix);

        // Get current user's UID and GID for container user mapping
        let user_uid = users::get_current_uid();
        let user_gid = users::get_current_gid();

        Ok(Self {
            dockerfile,
            container_name,
            image_name,
            engine_type,
            update_image: args.update,
            custom_command: args.command,
            lockfile,
            user_uid,
            user_gid,
        })
    }
}
