//! Container management utility
//!
//! This application provides a convenient way to create, manage, and enter container environments
//! using either Docker or Podman. It automatically searches for Dockerfiles, builds images when
//! needed, and provides seamless container lifecycle management.

use anyhow::{Context, Result};
use clap::Parser;
use std::env;

use std::path::PathBuf;

mod config;
mod container;
mod dockerfile;
mod engine;
mod errors;
mod lockfile;

use config::Config;
use container::ContainerEngine;
use lockfile::Lockfile;

/// Command-line arguments structure for the container management utility
#[derive(Parser)]
#[command(
    name = "containers",
    about = "Create or enter a container environment",
    after_help = "ENVIRONMENT VARIABLES:
  CONTAINER_NAME          Set default container name
  DOCKERFILE              Set default Dockerfile path
  CONTAINER_ENGINE        Container engine to use (default: podman)

EXAMPLES:
  containers                      Use default settings
  containers mycontainer          Use custom container name
  containers -f custom.dockerfile Use custom Dockerfile
  containers -u                   Update/rebuild image and container
  containers -- echo hello       Run custom command in container
  CONTAINER_ENGINE=docker containers    Use Docker instead of Podman"
)]
struct Args {
    /// Use specified Dockerfile (default: search current dir upward)
    #[arg(short, long, value_name = "PATH")]
    dockerfile: Option<PathBuf>,

    /// Rebuild image and recreate container
    #[arg(short, long)]
    update: bool,

    /// Name for the container (default: based on Dockerfile directory)
    #[arg(value_name = "CONTAINER_NAME")]
    container_name: Option<String>,

    /// Custom command to run in the container (after --)
    #[arg(last = true)]
    command: Vec<String>,
}

/// Main entry point for the container management utility
///
/// Parses command-line arguments, creates configuration, initializes the container engine,
/// and manages the complete container lifecycle.
fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::from_args_and_env(args)?;

    let engine = ContainerEngine::new(config.engine_type)?;

    run_container(&config, &engine).context("Failed to run container")
}

/// Orchestrates the container lifecycle based on configuration
///
/// This function handles:
/// - Checking lockfile for Dockerfile changes
/// - Building container images when needed, when update is requested, or when Dockerfile changed
/// - Creating new containers or entering existing ones
/// - Starting stopped containers
/// - Updating lockfile after successful builds
///
/// # Arguments
///
/// * `config` - Application configuration containing container settings
/// * `engine` - Container engine abstraction for executing container operations
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if any container operation fails.
fn run_container(config: &Config, engine: &ContainerEngine) -> Result<()> {
    // Build image if needed
    if config.dockerfile.exists() {
        // Load or create lockfile to check for Dockerfile changes
        let mut lockfile = Lockfile::load_or_create(&config.dockerfile)?;
        let dockerfile_changed = lockfile.has_dockerfile_changed(&config.dockerfile)?;
        
        let should_build = config.update_image 
            || !engine.image_exists(&config.image_name)? 
            || dockerfile_changed;

        if should_build {
            if config.update_image {
                println!("Updating image: {}", config.image_name);
            } else if dockerfile_changed {
                println!("Dockerfile changed, rebuilding image: {}", config.image_name);
            } else {
                println!("Building image: {}", config.image_name);
            }

            // Remove existing container if we're rebuilding due to changes
            if (config.update_image || dockerfile_changed) && engine.container_exists(&config.container_name)? {
                println!("Removing existing container: {}", config.container_name);
                engine.remove_container(&config.container_name)?;
            }

            engine.build_image(&config.image_name, &config.dockerfile)?;
            
            // Update lockfile with new Dockerfile state after successful build
            lockfile.update_dockerfile_info(&config.dockerfile)?;
            lockfile.save(&config.dockerfile)?;
        }
    }

    // Handle container lifecycle
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    if engine.container_exists(&config.container_name)? {
        if engine.container_running(&config.container_name)? {
            println!("Entering running container: {}", config.container_name);
            engine.exec_container(&config.container_name, &config.custom_command, &current_dir)?;
        } else {
            println!("Starting existing container: {}", config.container_name);
            engine.start_container(&config.container_name)?;
            engine.exec_container(&config.container_name, &config.custom_command, &current_dir)?;
        }
    } else {
        println!("Creating new container: {}", config.container_name);
        let mount_dir = config
            .dockerfile
            .parent()
            .context("Failed to get Dockerfile directory")?;
        engine.create_and_run_container(
            &config.container_name,
            &config.image_name,
            &mount_dir,
            &config.custom_command,
            &current_dir,
        )?;
    }

    Ok(())
}
