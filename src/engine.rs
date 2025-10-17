//! Container engine type definitions
//!
//! This module defines the supported container engines and provides
//! conversions between string representations and the typed enum.

use std::fmt;
use std::str::FromStr;

/// Supported container engine types
///
/// This enum represents the container engines that the application can work with.
/// Each variant corresponds to a specific container runtime with its own
/// command-line interface and behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    /// Docker container engine
    Docker,
    /// Podman container engine
    Podman,
}

impl EngineType {
    /// Returns the command name for this engine type
    ///
    /// This is the executable name that should be used when invoking
    /// the container engine from the command line.
    ///
    /// # Returns
    ///
    /// The string command name for the engine
    pub fn as_command(&self) -> &'static str {
        match self {
            EngineType::Docker => "docker",
            EngineType::Podman => "podman",
        }
    }
}

impl fmt::Display for EngineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_command())
    }
}

impl FromStr for EngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "docker" => Ok(EngineType::Docker),
            "podman" => Ok(EngineType::Podman),
            _ => Err(format!("Unknown engine type: {}", s)),
        }
    }
}

impl Default for EngineType {
    fn default() -> Self {
        EngineType::Podman
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!("docker".parse::<EngineType>().unwrap(), EngineType::Docker);
        assert_eq!("podman".parse::<EngineType>().unwrap(), EngineType::Podman);
        assert_eq!("DOCKER".parse::<EngineType>().unwrap(), EngineType::Docker);
        assert!("unknown".parse::<EngineType>().is_err());
    }

    #[test]
    fn test_as_command() {
        assert_eq!(EngineType::Docker.as_command(), "docker");
        assert_eq!(EngineType::Podman.as_command(), "podman");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", EngineType::Docker), "docker");
        assert_eq!(format!("{}", EngineType::Podman), "podman");
    }

    #[test]
    fn test_default() {
        assert_eq!(EngineType::default(), EngineType::Podman);
    }
}

