//! Error types for the generator library.
//!
//! This module provides a comprehensive error hierarchy using `thiserror`
//! for idiomatic error handling throughout the library.

use std::path::PathBuf;

use crate::serve::ServeError;

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

    /// Route-related errors
    #[error("Route error: {0}")]
    Route(#[from] RouteError),

    /// Build-related errors
    #[error("Build error: {0}")]
    Build(#[from] BuildError),

    /// Init-related errors
    #[error("Init error: {0}")]
    Init(#[from] InitError),

    /// Serve-related errors
    #[error("Serve error: {0}")]
    Serve(#[from] ServeError),

    /// Feed-related errors
    #[error("Feed error: {0}")]
    Feed(#[from] FeedError),

    /// I/O errors with context
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

macro_rules! impl_boxed_error {
    ($($error_type:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$error_type> for Box<GeneratorError> {
                fn from(err: $error_type) -> Self {
                    Box::new(GeneratorError::$variant(err))
                }
            }
        )*
    };
}

impl_boxed_error! {
    ConfigError => Config,
    ContentError => Content,
    TemplateError => Template,
    AssetError => Asset,
    RouteError => Route,
    BuildError => Build,
    InitError => Init,
    crate::serve::ServeError => Serve,
    FeedError => Feed,
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
    Syntax { template: String, message: String },

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
    MissingField { field: &'static str, path: PathBuf },

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

/// Route-related errors.
#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    /// Route not found
    #[error("Route not found: {0}")]
    NotFound(String),

    /// Duplicate route detected
    #[error("Duplicate route: {0}")]
    Duplicate(String),

    /// Invalid route path
    #[error("Invalid route path: {0}")]
    InvalidPath(String),

    /// Content discovery failed
    #[error("Content discovery failed: {0}")]
    DiscoveryFailed(String),
}

/// Build-related errors.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// No content found to build
    #[error("No content found to build")]
    NoContent,

    /// Output directory creation failed
    #[error("Failed to create output directory '{path}': {source}")]
    OutputDirCreation {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Page rendering failed
    #[error("Failed to render page '{path}': {source}")]
    PageRenderFailed {
        path: String,
        #[source]
        source: TemplateError,
    },

    /// Content processing failed
    #[error("Content processing failed: {0}")]
    ContentProcessing(#[from] ContentError),

    /// Asset processing failed
    #[error("Asset processing failed: {0}")]
    AssetProcessing(#[from] AssetError),

    /// Route discovery failed
    #[error("Route discovery failed: {0}")]
    RouteDiscovery(#[from] RouteError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Template error
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),

    /// I/O error with context
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Broken internal link
    #[error("Broken internal link in '{file}': target '{target}' not found")]
    BrokenInternalLink { file: String, target: String },
}

/// Init-related errors.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// Directory already exists and is not empty
    #[error("Directory is not empty: {0}")]
    DirectoryNotEmpty(PathBuf),

    /// Failed to create directory
    #[error("Failed to create directory '{path}': {source}")]
    DirectoryCreation {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to write file
    #[error("Failed to write file '{path}': {source}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// User cancelled the operation
    #[error("Operation cancelled by user")]
    Cancelled,

    /// Invalid site name
    #[error("Invalid site name: {0}")]
    InvalidName(String),

    /// Invalid base URL
    #[error("Invalid base URL: {0}")]
    InvalidBaseUrl(String),
}

/// Feed-related errors.
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    /// Feed generation failed
    #[error("Feed generation failed: {0}")]
    GenerationFailed(String),

    /// Invalid feed configuration
    #[error("Invalid feed configuration: {0}")]
    InvalidConfig(String),

    /// Missing required field
    #[error("Missing required field '{field}' in feed configuration")]
    MissingField { field: &'static str },

    /// I/O error writing feed
    #[error("I/O error writing feed to {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
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

    // RouteError tests

    #[test]
    fn test_route_error_not_found() {
        let err = RouteError::NotFound("/missing/".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Route not found"));
        assert!(msg.contains("/missing/"));
    }

    #[test]
    fn test_route_error_duplicate() {
        let err = RouteError::Duplicate("/about/".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Duplicate route"));
        assert!(msg.contains("/about/"));
    }

    #[test]
    fn test_route_error_invalid_path() {
        let err = RouteError::InvalidPath("missing-slashes".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid route path"));
        assert!(msg.contains("missing-slashes"));
    }

    #[test]
    fn test_route_error_discovery_failed() {
        let err = RouteError::DiscoveryFailed("Permission denied".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Content discovery failed"));
        assert!(msg.contains("Permission denied"));
    }

    #[test]
    fn test_generator_error_from_route() {
        let route_err = RouteError::NotFound("/test/".to_string());
        let gen_err: GeneratorError = route_err.into();
        assert!(matches!(gen_err, GeneratorError::Route(_)));
    }

    // BuildError tests

    #[test]
    fn test_build_error_no_content() {
        let err = BuildError::NoContent;
        let msg = err.to_string();
        assert!(msg.contains("No content found"));
    }

    #[test]
    fn test_build_error_output_dir_creation() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = BuildError::OutputDirCreation {
            path: PathBuf::from("dist"),
            source: io_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to create output directory"));
        assert!(msg.contains("dist"));
        assert!(msg.contains("access denied"));
    }

    #[test]
    fn test_build_error_page_render_failed() {
        let template_err = TemplateError::NotFound("page.html".to_string());
        let err = BuildError::PageRenderFailed {
            path: "/about/".to_string(),
            source: template_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to render page"));
        assert!(msg.contains("/about/"));
    }

    #[test]
    fn test_build_error_from_content() {
        let content_err = ContentError::NotFound(PathBuf::from("test.md"));
        let build_err: BuildError = content_err.into();
        assert!(matches!(build_err, BuildError::ContentProcessing(_)));
    }

    #[test]
    fn test_build_error_from_asset() {
        let asset_err = AssetError::NotFound(PathBuf::from("test.png"));
        let build_err: BuildError = asset_err.into();
        assert!(matches!(build_err, BuildError::AssetProcessing(_)));
    }

    #[test]
    fn test_build_error_from_route() {
        let route_err = RouteError::NotFound("/test/".to_string());
        let build_err: BuildError = route_err.into();
        assert!(matches!(build_err, BuildError::RouteDiscovery(_)));
    }

    #[test]
    fn test_build_error_from_config() {
        let config_err = ConfigError::NotFound(PathBuf::from("site.toml"));
        let build_err: BuildError = config_err.into();
        assert!(matches!(build_err, BuildError::Config(_)));
    }

    #[test]
    fn test_build_error_from_template() {
        let template_err = TemplateError::NotFound("test.html".to_string());
        let build_err: BuildError = template_err.into();
        assert!(matches!(build_err, BuildError::Template(_)));
    }

    #[test]
    fn test_generator_error_from_build() {
        let build_err = BuildError::NoContent;
        let gen_err: GeneratorError = build_err.into();
        assert!(matches!(gen_err, GeneratorError::Build(_)));
    }

    // InitError tests

    #[test]
    fn test_init_error_directory_not_empty() {
        let err = InitError::DirectoryNotEmpty(PathBuf::from("my-site"));
        let msg = err.to_string();
        assert!(msg.contains("Directory is not empty"));
        assert!(msg.contains("my-site"));
    }

    #[test]
    fn test_init_error_directory_creation() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = InitError::DirectoryCreation {
            path: PathBuf::from("my-site"),
            source: io_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to create directory"));
        assert!(msg.contains("my-site"));
    }

    #[test]
    fn test_init_error_file_write() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = InitError::FileWrite {
            path: PathBuf::from("my-site/site.toml"),
            source: io_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to write file"));
        assert!(msg.contains("site.toml"));
    }

    #[test]
    fn test_init_error_cancelled() {
        let err = InitError::Cancelled;
        let msg = err.to_string();
        assert!(msg.contains("cancelled"));
    }

    #[test]
    fn test_init_error_invalid_name() {
        let err = InitError::InvalidName("".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid site name"));
    }

    #[test]
    fn test_init_error_invalid_base_url() {
        let err = InitError::InvalidBaseUrl("not-a-url".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid base URL"));
    }

    #[test]
    fn test_generator_error_from_init() {
        let init_err = InitError::Cancelled;
        let gen_err: GeneratorError = init_err.into();
        assert!(matches!(gen_err, GeneratorError::Init(_)));
    }
}
