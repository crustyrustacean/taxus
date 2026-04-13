//! Error types for the generator library.
//!
//! This module provides a comprehensive error hierarchy using `thiserror`
//! for idiomatic error handling throughout the library.

use crate::serve::ServeError;
use std::path::PathBuf;

/// Result alias for generator operations.
pub type Result<T> = std::result::Result<T, GeneratorError>;

/// Main error type for the generator library.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Config(Box<ConfigError>),

    /// Content-related errors
    #[error("Content error: {0}")]
    Content(Box<ContentError>),

    /// Template-related errors
    #[error("Template error: {0}")]
    Template(Box<TemplateError>),

    /// Asset-related errors
    #[error("Asset error: {0}")]
    Asset(Box<AssetError>),

    /// Route-related errors
    #[error("Route error: {0}")]
    Route(Box<RouteError>),

    /// Init-related errors
    #[error("Init error: {0}")]
    Init(Box<InitError>),

    /// Serve-related errors
    #[error("Serve error: {0}")]
    Serve(Box<ServeError>),

    /// Feed-related errors
    #[error("Feed error: {0}")]
    Feed(Box<FeedError>),

    /// Image-related errors
    #[error("Image error: {0}")]
    Image(Box<ImageError>),

    /// WASM build-related errors
    #[error("WASM error: {0}")]
    Wasm(Box<WasmError>),

    /// I/O errors with context
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// No content found to build
    #[error("No content found to build")]
    NoContent,

    /// Broken internal link
    #[error("Broken internal link in '{file}': target '{target}' not found")]
    BrokenInternalLink { file: String, target: String },

    /// Page rendering failed
    #[error("Failed to render page '{path}': {source}")]
    PageRenderFailed {
        path: String,
        #[source]
        source: TemplateError,
    },
}

macro_rules! impl_from_for_generator_error {
    ($($error_type:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$error_type> for GeneratorError {
                fn from(err: $error_type) -> Self {
                    GeneratorError::$variant(Box::new(err))
                }
            }
        )*
    };
}

impl_from_for_generator_error! {
    ConfigError => Config,
    ContentError => Content,
    TemplateError => Template,
    AssetError => Asset,
    RouteError => Route,
    InitError => Init,
    ServeError => Serve,
    FeedError => Feed,
    ImageError => Image,
    WasmError => Wasm,
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

/// Image-related errors.
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// Image file not found
    #[error("Image not found: {0}")]
    NotFound(PathBuf),

    /// Failed to decode image
    #[error("Failed to decode image {path}: {reason}")]
    DecodeFailed { path: PathBuf, reason: String },

    /// Failed to encode image
    #[error("Failed to encode image: {0}")]
    EncodeFailed(String),

    /// Failed to resize image
    #[error("Failed to resize image: {0}")]
    ResizeFailed(String),

    /// I/O error with path context
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Invalid configuration
    #[error("Invalid image configuration: {0}")]
    InvalidConfig(String),
}

/// WASM build-related errors
#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    /// A required tool is not available
    #[error("{tool} not found. {hint}")]
    ToolMissing { tool: String, hint: String },

    /// The WASM build failed
    #[error("WASM build failed: {0}")]
    BuildFailed(String),
}

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
        let err = GeneratorError::NoContent;
        let msg = err.to_string();
        assert!(msg.contains("No content found"));
    }

    #[test]
    fn test_page_render_failed_error() {
        let template_err = TemplateError::NotFound("page.html".to_string());
        let err = GeneratorError::PageRenderFailed {
            path: "/about/".to_string(),
            source: template_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to render"));
        assert!(msg.contains("/about/"));
    }

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

    // ImageError tests

    #[test]
    fn test_image_error_not_found() {
        let err = ImageError::NotFound(PathBuf::from("hero.jpg"));
        let msg = err.to_string();
        assert!(msg.contains("Image not found"));
        assert!(msg.contains("hero.jpg"));
    }

    #[test]
    fn test_image_error_decode_failed() {
        let err = ImageError::DecodeFailed {
            path: PathBuf::from("hero.jpg"),
            reason: "invalid format".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to decode"));
        assert!(msg.contains("hero.jpg"));
    }

    #[test]
    fn test_generator_error_from_image() {
        let image_err = ImageError::NotFound(PathBuf::from("test.jpg"));
        let gen_err: GeneratorError = image_err.into();
        assert!(matches!(gen_err, GeneratorError::Image(_)));
    }
}
