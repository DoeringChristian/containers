use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContainerError {

    #[error("Failed to build image: {0}")]
    BuildFailed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

}

