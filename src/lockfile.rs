use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::config::{ContainerConfig, Dependency};

#[derive(Debug, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: String,
    pub containers: HashMap<String, ContainerLock>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerLock {
    pub image_hash: String,
    pub base_image: String,
    pub dependencies: Vec<DependencyLock>,
    pub config_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyLock {
    pub package: String,
    pub version: String,
    pub source: String,
    pub hash: String,
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            containers: HashMap::new(),
        }
    }
    
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read lockfile at {}", path.as_ref().display()))?;
        let lockfile = toml::from_str(&content)
            .context("Failed to parse lockfile")?;
        Ok(lockfile)
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize lockfile")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write lockfile at {}", path.as_ref().display()))?;
        Ok(())
    }
    
    pub fn generate_from_config(containers: &HashMap<String, ContainerConfig>) -> Result<Self> {
        let mut lockfile = Self::new();
        
        for (name, config) in containers {
            let config_hash = Self::hash_config(config);
            let base_image = config.base_image.as_deref().unwrap_or("ubuntu:latest");
            
            let dependencies = if let Some(deps) = &config.dependencies {
                deps.iter().map(|dep| {
                    DependencyLock {
                        package: dep.package.clone(),
                        version: dep.version.clone().unwrap_or_else(|| "latest".to_string()),
                        source: dep.source.clone().unwrap_or_else(|| "default".to_string()),
                        hash: Self::hash_dependency(dep),
                    }
                }).collect()
            } else {
                Vec::new()
            };
            
            lockfile.containers.insert(name.clone(), ContainerLock {
                image_hash: format!("{}-{}", name, &config_hash[..8]),
                base_image: base_image.to_string(),
                dependencies,
                config_hash,
            });
        }
        
        Ok(lockfile)
    }
    
    fn hash_config(config: &ContainerConfig) -> String {
        let serialized = toml::to_string(config).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    fn hash_dependency(dep: &Dependency) -> String {
        let mut hasher = Sha256::new();
        hasher.update(dep.package.as_bytes());
        if let Some(version) = &dep.version {
            hasher.update(version.as_bytes());
        }
        if let Some(source) = &dep.source {
            hasher.update(source.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
}