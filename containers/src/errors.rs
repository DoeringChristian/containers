//! Error types for container operations
//!
//! This module defines custom error types used throughout the container
//! management utility, providing structured error handling with descriptive
//! messages.

use thiserror::Error;

/// Errors that can occur during container operations
///
/// This enum represents all the container-specific errors that can occur
/// during the execution of container commands. Each variant provides
/// contextual information about what went wrong.
#[derive(Error, Debug)]
pub enum ContainerError {
    /// Image build operation failed
    ///
    /// This error occurs when a container image build process fails,
    /// typically due to Dockerfile issues, missing dependencies, or
    /// build context problems.
    #[error("Failed to build image: {0}")]
    BuildFailed(String),

    /// Container engine command execution failed
    ///
    /// This error occurs when a container engine command (docker/podman)
    /// returns a non-zero exit status, indicating the operation failed.
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
}
