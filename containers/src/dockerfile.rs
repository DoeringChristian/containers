use std::env;
use std::path::{Path, PathBuf};

pub struct DockerfileLocator;

impl DockerfileLocator {
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

