use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Container engine '{0}' not found")]
    EngineNotFound(String),

    #[error("Container '{0}' not found")]
    ContainerNotFound(String),

    #[error("Image '{0}' not found")]
    ImageNotFound(String),

    #[error("Failed to build image: {0}")]
    BuildFailed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("Dockerfile not found at '{0}'")]
    DockerfileNotFound(String),
}

