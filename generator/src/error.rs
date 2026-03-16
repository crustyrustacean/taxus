//! Error types for the generator library.
//!
//! This module provides a comprehensive error hierarchy using `thiserror`
//! for idiomatic error handling throughout the library.

use std::path::PathBuf;

/// Main error type for the generator library.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// I/O errors with context
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Configuration-related errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Configuration file not found
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),

    /// Invalid configuration content
    #[error("Invalid configuration: {0}")]
    Invalid(String),

    /// Failed to parse configuration file
    #[error("Failed to parse configuration: {0}")]
    Parse(#[from] toml::de::Error),

    /// Missing required field
    #[error("Missing required field '{field}' in configuration")]
    MissingField { field: &'static str },
}

/// Result alias for generator operations.
pub type Result<T> = std::result::Result<T, GeneratorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_not_found_display() {
        let err = ConfigError::NotFound(PathBuf::from("site.toml"));
        let msg = err.to_string();
        assert!(msg.contains("site.toml"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_config_error_invalid_display() {
        let err = ConfigError::Invalid("missing required field".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid configuration"));
        assert!(msg.contains("missing required field"));
    }

    #[test]
    fn test_config_error_from_toml() {
        let toml_err: toml::de::Error = toml::from_str::<toml::Value>("invalid[").unwrap_err();
        let config_err: ConfigError = toml_err.into();
        assert!(matches!(config_err, ConfigError::Parse(_)));
    }

    #[test]
    fn test_generator_error_from_config() {
        let config_err = ConfigError::NotFound(PathBuf::from("test.toml"));
        let gen_err: GeneratorError = config_err.into();
        assert!(matches!(gen_err, GeneratorError::Config(_)));
    }

    #[test]
    fn test_missing_field_error() {
        let err = ConfigError::MissingField { field: "site.name" };
        let msg = err.to_string();
        assert!(msg.contains("site.name"));
    }
}
