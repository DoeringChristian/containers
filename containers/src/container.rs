use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::errors::ContainerError;

pub struct ContainerEngine {
    engine_type: String,
    nvidia_args: Vec<String>,
}

impl ContainerEngine {
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

