//! Container engine abstraction
//!
//! This module provides a unified interface for interacting with container engines
//! like Docker and Podman. It handles engine-specific differences and provides
//! common operations for container lifecycle management.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::errors::ContainerError;

/// Container engine abstraction
///
/// Provides a unified interface for container operations that works with
/// both Docker and Podman. Automatically detects NVIDIA GPU support and
/// handles engine-specific argument differences.
pub struct ContainerEngine {
    /// The container engine type (docker or podman)
    engine_type: String,
    /// NVIDIA GPU support arguments for this engine
    nvidia_args: Vec<String>,
}

impl ContainerEngine {
    /// Creates a new container engine instance
    ///
    /// Verifies that the specified container engine is available on the system
    /// and automatically detects NVIDIA GPU support.
    ///
    /// # Arguments
    ///
    /// * `engine_type` - The container engine to use ("docker" or "podman")
    ///
    /// # Returns
    ///
    /// Returns a `Result<ContainerEngine>` or an error if the engine is not found.
    ///
    /// # Errors
    ///
    /// Will return an error if the specified container engine is not installed
    /// or not accessible in the system PATH.
    pub fn new(engine_type: &str) -> Result<Self> {
        // Verify engine exists
        which::which(engine_type)
            .with_context(|| format!("Container engine '{}' not found", engine_type))?;

        let nvidia_args = Self::detect_nvidia_support(engine_type);

        Ok(Self {
            engine_type: engine_type.to_string(),
            nvidia_args,
        })
    }

    /// Detects NVIDIA GPU support and returns appropriate arguments
    ///
    /// Checks if nvidia-smi is available and working, then returns the
    /// engine-specific arguments needed to enable GPU access in containers.
    ///
    /// # Arguments
    ///
    /// * `engine_type` - The container engine type ("docker" or "podman")
    ///
    /// # Returns
    ///
    /// A vector of arguments to pass to the container engine for GPU support,
    /// or an empty vector if no GPU support is detected.
    fn detect_nvidia_support(engine_type: &str) -> Vec<String> {
        let mut args = Vec::new();

        // Check if nvidia-smi exists and works
        if which::which("nvidia-smi").is_ok() {
            if let Ok(status) = Command::new("nvidia-smi")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                if status.success() {
                    match engine_type {
                        "docker" => {
                            args.push("--gpus".to_string());
                            args.push("all".to_string());
                        }
                        "podman" => {
                            args.push("--device".to_string());
                            args.push("nvidia.com/gpu=all".to_string());
                            args.push("--security-opt".to_string());
                            args.push("label=disable".to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        args
    }

    /// Checks if a container image exists locally
    ///
    /// # Arguments
    ///
    /// * `image_name` - The name of the image to check for
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the image exists, `Ok(false)` if it doesn't,
    /// or an error if the check fails.
    pub fn image_exists(&self, image_name: &str) -> Result<bool> {
        let output = Command::new(&self.engine_type)
            .arg("images")
            .arg("--format")
            .arg("table {{.Repository}}:{{.Tag}}")
            .output()
            .context("Failed to list images")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.lines().any(|line| {
            line.ends_with(image_name) || line.ends_with(&format!("localhost/{}", image_name))
        }))
    }

    /// Checks if a container exists (running or stopped)
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the container to check for
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the container exists, `Ok(false)` if it doesn't,
    /// or an error if the check fails.
    pub fn container_exists(&self, container_name: &str) -> Result<bool> {
        let output = Command::new(&self.engine_type)
            .arg("ps")
            .arg("-a")
            .arg("--format")
            .arg("table {{.Names}}")
            .output()
            .context("Failed to list containers")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.lines().any(|line| line == container_name))
    }

    /// Checks if a container is currently running
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the container to check
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the container is running, `Ok(false)` if it's not,
    /// or an error if the check fails.
    pub fn container_running(&self, container_name: &str) -> Result<bool> {
        let output = Command::new(&self.engine_type)
            .arg("ps")
            .arg("--format")
            .arg("table {{.Names}}")
            .output()
            .context("Failed to list running containers")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.lines().any(|line| line == container_name))
    }

    /// Removes a container forcefully
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the container to remove
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or an error if the removal fails.
    pub fn remove_container(&self, container_name: &str) -> Result<()> {
        let status = Command::new(&self.engine_type)
            .arg("rm")
            .arg("-f")
            .arg(container_name)
            .status()
            .context("Failed to remove container")?;

        if !status.success() {
            return Err(ContainerError::CommandFailed(format!("rm -f {}", container_name)).into());
        }
        Ok(())
    }

    /// Builds a container image from a Dockerfile
    ///
    /// # Arguments
    ///
    /// * `image_name` - The name to tag the built image with
    /// * `dockerfile` - Path to the Dockerfile to build from
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or an error if the build fails.
    pub fn build_image(&self, image_name: &str, dockerfile: &Path) -> Result<()> {
        let status = Command::new(&self.engine_type)
            .arg("build")
            .arg("-t")
            .arg(image_name)
            .arg("-f")
            .arg(dockerfile)
            .arg(".")
            .status()
            .context("Failed to build image")?;

        if !status.success() {
            return Err(ContainerError::BuildFailed(image_name.to_string()).into());
        }
        Ok(())
    }

    /// Starts a stopped container
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the container to start
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or an error if starting fails.
    pub fn start_container(&self, container_name: &str) -> Result<()> {
        let status = Command::new(&self.engine_type)
            .arg("start")
            .arg(container_name)
            .status()
            .context("Failed to start container")?;

        if !status.success() {
            return Err(ContainerError::CommandFailed(format!("start {}", container_name)).into());
        }
        Ok(())
    }

    /// Executes a bash shell in a running container
    ///
    /// This method creates an interactive bash session inside the specified
    /// container, allowing the user to interact with the container directly.
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the running container to exec into
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the shell session ends, or an error if exec fails.
    pub fn exec_container(&self, container_name: &str) -> Result<()> {
        let status = Command::new(&self.engine_type)
            .arg("exec")
            .arg("-it")
            .arg(container_name)
            .arg("/bin/bash")
            .status()
            .context("Failed to exec into container")?;

        if !status.success() {
            return Err(ContainerError::CommandFailed(format!(
                "exec -it {} /bin/bash",
                container_name
            ))
            .into());
        }
        Ok(())
    }

    /// Creates and runs a new container with the specified configuration
    ///
    /// This method creates a new container with:
    /// - Interactive TTY allocation
    /// - Current directory mounted as a volume at the same path in the container
    /// - Working directory set to the current directory
    /// - NVIDIA GPU support if available
    /// - Automatic execution of /bin/bash
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name for the new container
    /// * `image_name` - The container image to use
    /// * `current_dir` - The current working directory to mount and use
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the container session ends, or an error if creation/running fails.
    pub fn create_and_run_container(
        &self,
        container_name: &str,
        image_name: &str,
        current_dir: &Path,
    ) -> Result<()> {
        let mut cmd = Command::new(&self.engine_type);
        cmd.arg("run")
            .arg("-it")
            .arg("--name")
            .arg(container_name)
            .arg("-v")
            .arg(format!(
                "{}:{}",
                current_dir.display(),
                current_dir.display()
            ))
            .arg("-w")
            .arg(current_dir);

        // Add NVIDIA arguments
        for arg in &self.nvidia_args {
            cmd.arg(arg);
        }

        cmd.arg(image_name).arg("/bin/bash");

        let status = cmd.status().context("Failed to create and run container")?;

        if !status.success() {
            return Err(
                ContainerError::CommandFailed(format!("run container {}", container_name)).into(),
            );
        }
        Ok(())
    }
}
