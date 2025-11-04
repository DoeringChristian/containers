use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use crate::config::ContainerConfig;
use crate::lockfile::ContainerLock;

pub struct DockerfileGenerator;

impl DockerfileGenerator {
    pub fn generate(config: &ContainerConfig, lock: &ContainerLock) -> String {
        let mut dockerfile = String::new();
        
        dockerfile.push_str(&format!("FROM {}\n\n", lock.base_image));
        
        dockerfile.push_str("# Install system dependencies\n");
        dockerfile.push_str("RUN apt-get update && apt-get install -y \\\n");
        dockerfile.push_str("    sudo \\\n");
        dockerfile.push_str("    && rm -rf /var/lib/apt/lists/*\n\n");
        
        if !lock.dependencies.is_empty() {
            dockerfile.push_str("# Install dependencies\n");
            for dep in &lock.dependencies {
                match dep.source.as_str() {
                    "apt" => {
                        dockerfile.push_str(&format!(
                            "RUN apt-get update && apt-get install -y {} && rm -rf /var/lib/apt/lists/*\n",
                            if dep.version != "latest" {
                                format!("{}={}", dep.package, dep.version)
                            } else {
                                dep.package.clone()
                            }
                        ));
                    }
                    "pip" => {
                        dockerfile.push_str(&format!(
                            "RUN pip install {}",
                            if dep.version != "latest" {
                                format!("{}=={}", dep.package, dep.version)
                            } else {
                                dep.package.clone()
                            }
                        ));
                        dockerfile.push_str("\n");
                    }
                    _ => {
                        dockerfile.push_str(&format!("# TODO: Install {} from {}\n", dep.package, dep.source));
                    }
                }
            }
            dockerfile.push_str("\n");
        }
        
        if let Some(env_vars) = &config.environment {
            dockerfile.push_str("# Set environment variables\n");
            for (key, value) in env_vars {
                dockerfile.push_str(&format!("ENV {}={}\n", key, value));
            }
            dockerfile.push_str("\n");
        }
        
        dockerfile.push_str("# Create user\n");
        dockerfile.push_str("ARG UID=1000\n");
        dockerfile.push_str("ARG GID=1000\n");
        dockerfile.push_str("RUN groupadd -g $GID code && \\\n");
        dockerfile.push_str("    useradd -m -u $UID -g $GID -s /bin/bash code && \\\n");
        dockerfile.push_str("    usermod -aG sudo code && \\\n");
        dockerfile.push_str("    echo 'code ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers\n\n");
        
        dockerfile.push_str("# Copy and set entrypoint\n");
        dockerfile.push_str("COPY entrypoint.sh /entrypoint.sh\n");
        dockerfile.push_str("RUN chmod +x /entrypoint.sh\n\n");
        
        dockerfile.push_str("USER code\n");
        dockerfile.push_str("WORKDIR /home/code/work\n\n");
        
        dockerfile.push_str("ENTRYPOINT [\"/entrypoint.sh\"]\n");
        
        if let Some(cmd) = &config.command {
            dockerfile.push_str(&format!("CMD [{}]\n", 
                cmd.iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        
        dockerfile
    }
    
    pub fn save<P: AsRef<Path>>(dockerfile_content: &str, path: P) -> Result<()> {
        fs::write(&path, dockerfile_content)
            .with_context(|| format!("Failed to write Dockerfile to {}", path.as_ref().display()))?;
        Ok(())
    }
    
    pub fn generate_entrypoint() -> String {
        r#"#!/bin/bash
set -e

# Update UID/GID if needed
if [ ! -z "$UID" ] && [ "$UID" != "$(id -u)" ]; then
    sudo usermod -u $UID code
fi

if [ ! -z "$GID" ] && [ "$GID" != "$(id -g)" ]; then
    sudo groupmod -g $GID code
    sudo usermod -g $GID code
fi

# Fix ownership of home directory
sudo chown -R code:code /home/code

exec "$@"
"#.to_string()
    }
}