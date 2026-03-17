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

    /// Content-related errors
    #[error("Content error: {0}")]
    Content(#[from] ContentError),

    /// Template-related errors
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),

    /// Asset-related errors
    #[error("Asset error: {0}")]
    Asset(#[from] AssetError),

    /// I/O errors with context
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Template-related errors.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// Template file not found
    #[error("Template not found: {0}")]
    NotFound(String),

    /// Template rendering failed
    #[error("Template rendering failed: {0}")]
    Render(String),

    /// Invalid template syntax
    #[error("Invalid template syntax in {template}: {message}")]
    Syntax {
        template: String,
        message: String,
    },

    /// I/O error reading template
    #[error("I/O error reading template {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Template directory not found
    #[error("Template directory not found: {0}")]
    DirNotFound(PathBuf),
}

/// Content-related errors.
#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    /// Content file not found
    #[error("Content file not found: {0}")]
    NotFound(PathBuf),

    /// Invalid frontmatter in content file
    #[error("Invalid frontmatter in {path}: {source}")]
    InvalidFrontmatter {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    /// Unclosed frontmatter delimiter
    #[error("Unclosed frontmatter in {0}")]
    UnclosedFrontmatter(PathBuf),

    /// I/O error reading content file
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Missing required field in frontmatter
    #[error("Missing required field '{field}' in {path}")]
    MissingField {
        field: &'static str,
        path: PathBuf,
    },

    /// Invalid content path
    #[error("Invalid content path: {0}")]
    InvalidPath(String),
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

/// Asset-related errors.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    /// Asset file not found
    #[error("Asset not found: {0}")]
    NotFound(PathBuf),

    /// SCSS compilation error
    #[error("SCSS compilation error: {0}")]
    Scss(String),

    /// I/O error with path context
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// File copy failure
    #[error("Failed to copy from '{src}' to '{dest}': {reason}")]
    CopyFailed {
        src: PathBuf,
        dest: PathBuf,
        reason: String,
    },
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

    // ContentError tests

    #[test]
    fn test_content_error_not_found_display() {
        let err = ContentError::NotFound(PathBuf::from("content/about.md"));
        let msg = err.to_string();
        assert!(msg.contains("content/about.md"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_content_error_invalid_frontmatter() {
        let toml_err: toml::de::Error = toml::from_str::<toml::Value>("invalid[").unwrap_err();
        let err = ContentError::InvalidFrontmatter {
            path: PathBuf::from("content/test.md"),
            source: toml_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Invalid frontmatter"));
        assert!(msg.contains("content/test.md"));
    }

    #[test]
    fn test_content_error_missing_field() {
        let err = ContentError::MissingField {
            field: "title",
            path: PathBuf::from("content/test.md"),
        };
        let msg = err.to_string();
        assert!(msg.contains("title"));
        assert!(msg.contains("content/test.md"));
    }

    #[test]
    fn test_generator_error_from_content() {
        let content_err = ContentError::NotFound(PathBuf::from("test.md"));
        let gen_err: GeneratorError = content_err.into();
        assert!(matches!(gen_err, GeneratorError::Content(_)));
    }

    // TemplateError tests

    #[test]
    fn test_template_error_not_found() {
        let err = TemplateError::NotFound("missing.html".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Template not found"));
        assert!(msg.contains("missing.html"));
    }

    #[test]
    fn test_template_error_render() {
        let err = TemplateError::Render("Failed to render".to_string());
        let msg = err.to_string();
        assert!(msg.contains("rendering failed"));
    }

    #[test]
    fn test_template_error_syntax() {
        let err = TemplateError::Syntax {
            template: "bad.html".to_string(),
            message: "Unclosed tag".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Invalid template syntax"));
        assert!(msg.contains("bad.html"));
        assert!(msg.contains("Unclosed tag"));
    }

    #[test]
    fn test_template_error_dir_not_found() {
        let err = TemplateError::DirNotFound(PathBuf::from("templates"));
        let msg = err.to_string();
        assert!(msg.contains("Template directory not found"));
        assert!(msg.contains("templates"));
    }

    #[test]
    fn test_generator_error_from_template() {
        let template_err = TemplateError::NotFound("test.html".to_string());
        let gen_err: GeneratorError = template_err.into();
        assert!(matches!(gen_err, GeneratorError::Template(_)));
    }

    // AssetError tests

    #[test]
    fn test_asset_error_not_found() {
        let err = AssetError::NotFound(PathBuf::from("static/missing.png"));
        let msg = err.to_string();
        assert!(msg.contains("Asset not found"));
        assert!(msg.contains("static/missing.png"));
    }

    #[test]
    fn test_asset_error_scss() {
        let err = AssetError::Scss("Invalid syntax at line 5".to_string());
        let msg = err.to_string();
        assert!(msg.contains("SCSS compilation error"));
        assert!(msg.contains("Invalid syntax at line 5"));
    }

    #[test]
    fn test_asset_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = AssetError::Io {
            path: PathBuf::from("static/file.txt"),
            source: io_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("I/O error"));
        assert!(msg.contains("static/file.txt"));
        assert!(msg.contains("access denied"));
    }

    #[test]
    fn test_asset_error_copy_failed() {
        let err = AssetError::CopyFailed {
            src: PathBuf::from("static/file.txt"),
            dest: PathBuf::from("dist/static/file.txt"),
            reason: "Permission denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to copy"));
        assert!(msg.contains("static/file.txt"));
        assert!(msg.contains("dist/static/file.txt"));
        assert!(msg.contains("Permission denied"));
    }

    #[test]
    fn test_generator_error_from_asset() {
        let asset_err = AssetError::NotFound(PathBuf::from("test.png"));
        let gen_err: GeneratorError = asset_err.into();
        assert!(matches!(gen_err, GeneratorError::Asset(_)));
    }
}
