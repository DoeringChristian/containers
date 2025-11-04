use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub base_image: Option<String>,
    pub command: Option<Vec<String>>,
    pub environment: Option<HashMap<String, String>>,
    pub volumes: Option<Vec<VolumeMount>>,
    pub tmpfs: Option<Vec<TmpfsMount>>,
    pub dependencies: Option<Vec<Dependency>>,
    pub gpu: Option<bool>,
    pub interactive: Option<bool>,
    pub tty: Option<bool>,
    pub remove: Option<bool>,
    pub build_context: Option<BuildContext>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    pub read_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmpfsMount {
    pub target: String,
    pub size: Option<String>,
    pub read_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    pub package: String,
    pub version: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildContext {
    pub dockerfile_path: Option<PathBuf>,
    pub context_path: Option<PathBuf>,
    pub build_args: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainersToml {
    pub containers: HashMap<String, ContainerConfig>,
}

impl ContainersToml {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.as_ref().display()))?;
        let config = toml::from_str(&content)
            .context("Failed to parse containers.toml")?;
        Ok(config)
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.as_ref().display()))?;
        Ok(())
    }
    
    pub fn get_container(&self, name: &str) -> Option<&ContainerConfig> {
        self.containers.get(name)
    }
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            base_image: Some("claude".to_string()),
            command: None,
            environment: None,
            volumes: None,
            tmpfs: None,
            dependencies: None,
            gpu: Some(false),
            interactive: Some(true),
            tty: Some(true),
            remove: Some(true),
            build_context: None,
        }
    }
}