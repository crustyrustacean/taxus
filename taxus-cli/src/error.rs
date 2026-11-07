// src/generator/bin/taxus/error.rs

use std::path::PathBuf;
use taxus::error::{ConfigError, GeneratorError, TemplateError};

use crate::serve_error::ServeError;

/// Errors produced while scaffolding a new site.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// Directory already exists and is not empty.
    #[error("Directory is not empty: {0}")]
    DirectoryNotEmpty(PathBuf),
    /// Failed to create a directory.
    #[error("Failed to create directory '{path}': {source}")]
    DirectoryCreation {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Failed to write a file.
    #[error("Failed to write file '{path}': {source}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The user cancelled initialization.
    #[error("Operation cancelled by user")]
    Cancelled,
    /// The site name is invalid.
    #[error("Invalid site name: {0}")]
    InvalidName(String),
    /// The base URL is invalid.
    #[error("Invalid base URL: {0}")]
    InvalidBaseUrl(String),
}

/// Result type for site scaffolding operations.
pub type InitResult<T> = std::result::Result<T, InitError>;

/// Errors returned while executing a product command.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// An error from the reusable SSG library.
    #[error(transparent)]
    Generator(#[from] GeneratorError),
    /// An error from the development server.
    #[error("Serve error: {0}")]
    Serve(#[from] ServeError),
    /// An error from site scaffolding.
    #[error(transparent)]
    Init(#[from] InitError),
}

// ---------------------------------------------------------------------------
// Error rendering
// ---------------------------------------------------------------------------

/// Print a user-friendly error message with a contextual hint.
pub fn render_error(e: &GeneratorError) {
    tracing::error!(error = %e, error_type = std::any::type_name_of_val(e), "Build failed");

    eprintln!("\n✗ Error: {e}");

    let hint: Option<&str> = match e {
        GeneratorError::Config(inner) if matches!(**inner, ConfigError::NotFound(_)) => Some(
            "Run 'taxus init' to create a new site, or use --dir to point to your site directory.",
        ),
        GeneratorError::NoContent => {
            Some("Add .md files to your content/ directory. Start with content/_index.md.")
        }
        GeneratorError::Template(inner) if matches!(**inner, TemplateError::NotFound(_)) => Some(
            "Check that your templates/ directory exists and contains base.html and page.html.",
        ),
        GeneratorError::Template(inner) if matches!(**inner, TemplateError::DirNotFound(_)) => {
            Some(
                "Check that your templates/ directory exists. Run 'taxus init' to create a default site.",
            )
        }
        _ => None,
    };

    if let Some(hint) = hint {
        tracing::warn!(hint = hint, "Suggestion");
        tracing::error!("  Hint: {hint}");
    }
}

/// Render either a library or development-server command error.
pub fn render_command_error(error: &CommandError) {
    match error {
        CommandError::Generator(error) => render_error(error),
        CommandError::Serve(error) => {
            tracing::error!(error = %error, error_type = std::any::type_name_of_val(error), "Serve failed");
            eprintln!("\n✗ Error: Serve error: {error}");
        }
        CommandError::Init(InitError::Cancelled) => {}
        CommandError::Init(error) => {
            tracing::error!(error = %error, error_type = std::any::type_name_of_val(error), "Init failed");
            eprintln!("\n✗ Error: Init error: {error}");
        }
    }
}
