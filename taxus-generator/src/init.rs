// generator/src/init/init.rs

//! Site initialization module.
//!
//! This module provides functionality for scaffolding new static sites
//! with a default directory structure and configuration files.

mod scaffold;

use crate::error::{GeneratorError, InitError, Result};
pub use scaffold::InitScaffolder;
use std::path::{Path, PathBuf};
use tracing::info;

/// Options for initializing a new site.
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// Site name (used in configuration and templates)
    pub name: String,
    /// Base URL for the site
    pub base_url: String,
    /// Force initialization even if directory is not empty
    pub force: bool,
    /// Include islands support (Yew/WASM hydration)
    pub islands: bool,
}

impl Default for InitOptions {
    fn default() -> Self {
        Self {
            name: "My Site".to_string(),
            base_url: "https://example.com".to_string(),
            force: false,
            islands: false,
        }
    }
}

impl InitOptions {
    /// Create new init options with the given name and base URL.
    pub fn new(name: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            force: false,
            islands: false,
        }
    }

    /// Set the force flag.
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Set the islands flag.
    pub fn with_islands(mut self, islands: bool) -> Self {
        self.islands = islands;
        self
    }

    /// Validate the options.
    pub fn validate(&self) -> std::result::Result<(), InitError> {
        if self.name.trim().is_empty() {
            return Err(InitError::InvalidName(
                "Site name cannot be empty".to_string(),
            ));
        }

        if self.base_url.trim().is_empty() {
            return Err(InitError::InvalidBaseUrl(
                "Base URL cannot be empty".to_string(),
            ));
        }

        // Basic URL validation - must start with http:// or https://
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(InitError::InvalidBaseUrl(
                "Base URL must start with http:// or https://".to_string(),
            ));
        }

        Ok(())
    }
}

/// Report generated after site initialization.
#[derive(Debug, Clone)]
pub struct InitReport {
    /// Path where the site was initialized
    pub path: PathBuf,
    /// Number of directories created
    pub directories_created: usize,
    /// Number of files created
    pub files_created: usize,
    /// List of directories that were created
    pub created_dirs: Vec<PathBuf>,
    /// List of files that were created
    pub created_files: Vec<PathBuf>,
}

impl InitReport {
    /// Create a new init report.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            directories_created: 0,
            files_created: 0,
            created_dirs: Vec::new(),
            created_files: Vec::new(),
        }
    }

    /// Print a summary of the initialization.
    pub fn print_summary(&self) {
        // Emit structured log
        info!(
            path = %self.path.display(),
            directories = self.directories_created,
            files = self.files_created,
            "Site initialized"
        );

        // Human-readable output
        info!("\n✓ Site initialized at {}/\n", self.path.display());

        if !self.created_dirs.is_empty() {
            info!("  Directories");
            for dir in &self.created_dirs {
                info!("    {}/", dir.display());
            }
        }

        if !self.created_files.is_empty() {
            info!("  Files");
            for file in &self.created_files {
                info!("    {}", file.display());
            }
        }

        info!("Next steps:");
        info!("  cd {}", self.path.display());
        info!("  Edit site.toml to set your site name and base URL");
        info!("  Add content to the content/ directory");
        info!("  Customize templates in templates/");
        info!("  Run: yew-ssg serve --open");
    }
}

/// Check if a directory is empty.
///
/// Returns `Ok(true)` if the directory is empty or doesn't exist,
/// `Ok(false)` if it contains files, or an error if unable to read.
pub fn is_directory_empty(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }

    if !path.is_dir() {
        return Err(GeneratorError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Path is not a directory",
            ),
        });
    }

    let mut entries = std::fs::read_dir(path).map_err(|e| GeneratorError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    Ok(entries.next().is_none())
}

/// Derive a site name from a directory path.
///
/// Uses the directory name, or "My Site" if the path is "." or empty.
pub fn derive_site_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| *name != ".")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "My Site".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_options_default() {
        let opts = InitOptions::default();
        assert_eq!(opts.name, "My Site");
        assert_eq!(opts.base_url, "https://example.com");
        assert!(!opts.force);
    }

    #[test]
    fn test_init_options_new() {
        let opts = InitOptions::new("Test Site", "https://test.com");
        assert_eq!(opts.name, "Test Site");
        assert_eq!(opts.base_url, "https://test.com");
        assert!(!opts.force);
    }

    #[test]
    fn test_init_options_with_force() {
        let opts = InitOptions::default().with_force(true);
        assert!(opts.force);
    }

    #[test]
    fn test_init_options_validate_valid() {
        let opts = InitOptions::new("Test", "https://example.com");
        assert!(opts.validate().is_ok());
    }

    #[test]
    fn test_init_options_validate_empty_name() {
        let opts = InitOptions::new("", "https://example.com");
        assert!(matches!(opts.validate(), Err(InitError::InvalidName(_))));
    }

    #[test]
    fn test_init_options_validate_whitespace_name() {
        let opts = InitOptions::new("   ", "https://example.com");
        assert!(matches!(opts.validate(), Err(InitError::InvalidName(_))));
    }

    #[test]
    fn test_init_options_validate_empty_base_url() {
        let opts = InitOptions::new("Test", "");
        assert!(matches!(opts.validate(), Err(InitError::InvalidBaseUrl(_))));
    }

    #[test]
    fn test_init_options_validate_invalid_url_scheme() {
        let opts = InitOptions::new("Test", "ftp://example.com");
        assert!(matches!(opts.validate(), Err(InitError::InvalidBaseUrl(_))));
    }

    #[test]
    fn test_init_options_validate_missing_scheme() {
        let opts = InitOptions::new("Test", "example.com");
        assert!(matches!(opts.validate(), Err(InitError::InvalidBaseUrl(_))));
    }

    #[test]
    fn test_init_report_new() {
        let report = InitReport::new(PathBuf::from("my-site"));
        assert_eq!(report.path, PathBuf::from("my-site"));
        assert_eq!(report.directories_created, 0);
        assert_eq!(report.files_created, 0);
        assert!(report.created_dirs.is_empty());
        assert!(report.created_files.is_empty());
    }

    #[test]
    fn test_is_directory_empty_nonexistent() {
        let result = is_directory_empty(Path::new("nonexistent_directory_12345"));
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_is_directory_empty_with_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let result = is_directory_empty(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_is_directory_empty_with_files() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
        let result = is_directory_empty(temp_dir.path());
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_is_directory_empty_with_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();
        let result = is_directory_empty(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_site_name_from_path() {
        assert_eq!(derive_site_name(Path::new("my-site")), "my-site");
        assert_eq!(derive_site_name(Path::new("/path/to/my-site")), "my-site");
        assert_eq!(derive_site_name(Path::new(".")), "My Site");
    }
}
