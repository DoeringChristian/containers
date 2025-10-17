use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

use crate::dockerfile::DockerfileLocator;
use crate::Args;

#[derive(Debug)]
pub struct Config {
    pub dockerfile: PathBuf,
    pub container_name: String,
    pub image_name: String,
    pub engine_type: String,
    pub update_image: bool,
}

impl Config {
    pub fn from_args_and_env(args: Args) -> Result<Self> {
        let engine_type = env::var("CONTAINER_ENGINE").unwrap_or_else(|_| "podman".to_string());
        
        // Find Dockerfile
        let dockerfile = if let Some(dockerfile) = args.dockerfile {
            dockerfile
        } else if let Ok(dockerfile) = env::var("DOCKERFILE") {
            PathBuf::from(dockerfile)
        } else {
            DockerfileLocator::find().unwrap_or_else(|| {
                let exe_path = env::current_exe().unwrap_or_default();
                let exe_dir = exe_path.parent().unwrap_or_else(|| std::path::Path::new("."));
                exe_dir.join("Dockerfile")
            })
        };

        // Set container name
        let default_container_name = generate_container_name(&dockerfile);
        let container_name = if let Some(name) = args.container_name {
            name
        } else {
            env::var("CONTAINER_NAME").unwrap_or(default_container_name)
        };

        Ok(Self {
            dockerfile,
            container_name,
            image_name: "dev-env:latest".to_string(),
            engine_type,
            update_image: args.update,
        })
    }
}

fn generate_container_name(dockerfile: &std::path::Path) -> String {
    let dir = dockerfile.parent().unwrap_or_else(|| std::path::Path::new("."));
    let path_str = dir.to_string_lossy();
    
    // Remove leading slash and replace slashes with dashes
    path_str
        .strip_prefix('/')
        .unwrap_or(&path_str)
        .replace('/', "-")
}