// taxus-generator/src/config.rs

//! Configuration types for the generator.
//!
//! This module provides types for loading and representing site configuration
//! from `site.toml` files.

use crate::error::{ConfigError, GeneratorError, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Site configuration loaded from site.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    /// Site metadata
    pub site: SiteMeta,
    /// Build configuration
    #[serde(default)]
    pub build: BuildConfig,
    /// Feed configuration
    #[serde(default)]
    pub feed: FeedConfig,
    // Syntax highlighting configuration
    #[serde(default)]
    pub highlight: HighlightConfig,
    /// Base directory containing site.toml (not serialized)
    #[serde(skip)]
    pub base_dir: PathBuf,
}

/// Site metadata from the [site] section.
#[derive(Debug, Clone, Deserialize)]
pub struct SiteMeta {
    /// Site name/title
    pub name: String,
    /// Base URL for the site
    pub base_url: String,
    /// Optional site description
    pub description: Option<String>,
    /// Optional author name
    pub author: Option<String>,
}

/// Build configuration from the [build] section.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildConfig {
    /// Content directory path
    #[serde(default = "default_content_dir")]
    pub content_dir: PathBuf,

    /// Output directory path
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,

    /// Static files directory path
    #[serde(default = "default_static_dir")]
    pub static_dir: PathBuf,

    /// Styles directory path
    #[serde(default = "default_styles_dir")]
    pub styles_dir: PathBuf,

    /// Templates directory path
    #[serde(default = "default_templates_dir")]
    pub templates_dir: PathBuf,
}

fn default_content_dir() -> PathBuf {
    PathBuf::from("content")
}
fn default_output_dir() -> PathBuf {
    PathBuf::from("dist")
}
fn default_static_dir() -> PathBuf {
    PathBuf::from("static")
}
fn default_styles_dir() -> PathBuf {
    PathBuf::from("styles")
}
fn default_templates_dir() -> PathBuf {
    PathBuf::from("templates")
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            content_dir: default_content_dir(),
            output_dir: default_output_dir(),
            static_dir: default_static_dir(),
            styles_dir: default_styles_dir(),
            templates_dir: default_templates_dir(),
        }
    }
}

/// Feed configuration from the [feed] section.
#[derive(Debug, Clone, Deserialize)]
pub struct FeedConfig {
    /// Enable RSS feed generation
    #[serde(default = "default_rss_enabled")]
    pub rss_enabled: bool,

    /// Enable Atom feed generation
    #[serde(default = "default_atom_enabled")]
    pub atom_enabled: bool,

    /// Number of entries to include in feeds (0 = all)
    #[serde(default)]
    pub limit: usize,

    /// Include full content in feeds (vs summaries)
    #[serde(default = "default_full_content")]
    pub full_content: bool,

    /// Custom feed title (defaults to site name)
    pub title: Option<String>,

    /// Custom feed path (defaults to "feed.xml" for RSS, "atom.xml" for Atom)
    pub rss_path: Option<String>,

    /// Custom Atom feed path
    pub atom_path: Option<String>,
}

fn default_rss_enabled() -> bool {
    true
}

fn default_atom_enabled() -> bool {
    false
}

fn default_full_content() -> bool {
    false
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            rss_enabled: default_rss_enabled(),
            atom_enabled: default_atom_enabled(),
            limit: 0,
            full_content: default_full_content(),
            title: None,
            rss_path: None,
            atom_path: None,
        }
    }
}

/// Highlight configuration from the [highlight] section.
#[derive(Debug, Clone, Deserialize)]
pub struct HighlightConfig {
    /// Enable syntax highlighting
    #[serde(default = "default_highlight_enabled")]
    pub enabled: bool,

    /// CSS class prefix for highlight spans
    #[serde(default = "default_class_prefix")]
    pub class_prefix: String,
}

fn default_highlight_enabled() -> bool {
    true
}

fn default_class_prefix() -> String {
    "hl-".to_string()
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            enabled: default_highlight_enabled(),
            class_prefix: default_class_prefix(),
        }
    }
}

impl BuildConfig {
    /// Resolve all relative paths to be absolute paths based on the base directory.
    ///
    /// This ensures that paths work correctly regardless of the current working directory.
    pub fn resolve_paths(&mut self, base_dir: &Path) {
        self.content_dir = Self::resolve_path(&self.content_dir, base_dir);
        self.output_dir = Self::resolve_path(&self.output_dir, base_dir);
        self.static_dir = Self::resolve_path(&self.static_dir, base_dir);
        self.styles_dir = Self::resolve_path(&self.styles_dir, base_dir);
        self.templates_dir = Self::resolve_path(&self.templates_dir, base_dir);
    }

    /// Resolve a single path relative to the base directory.
    ///
    /// Absolute paths are preserved as-is.
    fn resolve_path(path: &Path, base_dir: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            base_dir.join(path)
        }
    }
}

impl SiteConfig {
    /// Load configuration from a site.toml file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use taxus_lib::config::SiteConfig;
    ///
    /// let config = SiteConfig::from_file("site.toml")?;
    /// println!("Site name: {}", config.site.name);
    /// # Ok::<(), taxus_lib::error::GeneratorError>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_path_buf()).into());
        }

        // Get the directory containing the config file as the base directory
        let base_dir = path
            .parent()
            .ok_or_else(|| {
                ConfigError::Invalid(format!(
                    "Cannot determine parent directory of config file: {}",
                    path.display()
                ))
            })?
            .to_path_buf();

        let content = std::fs::read_to_string(path).map_err(|e| GeneratorError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut config: Self = toml::from_str(&content).map_err(ConfigError::from)?;

        // Resolve all relative paths to be absolute based on the config file location
        config.build.resolve_paths(&base_dir);
        config.base_dir = base_dir;

        Ok(config)
    }

    /// Load configuration from a directory containing site.toml.
    ///
    /// Looks for `site.toml` in the given directory.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use taxus_lib::config::SiteConfig;
    ///
    /// let config = SiteConfig::from_dir("./mysite")?;
    /// # Ok::<(), taxus_lib::error::GeneratorError>(())
    /// ```
    pub fn from_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let config_path = dir.as_ref().join("site.toml");
        Self::from_file(config_path)
    }

    /// Create a new configuration with the given name and base URL.
    ///
    /// Uses default build and feed configuration.
    pub fn new(name: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            site: SiteMeta {
                name: name.into(),
                base_url: base_url.into(),
                description: None,
                author: None,
            },
            build: BuildConfig::default(),
            feed: FeedConfig::default(),
            highlight: HighlightConfig::default(),
            base_dir: PathBuf::new(),
        }
    }

    /// Validate the configuration.
    ///
    /// Returns an error if required fields are missing or invalid.
    pub fn validate(&self) -> Result<()> {
        if self.site.name.is_empty() {
            return Err(ConfigError::MissingField { field: "site.name" }.into());
        }

        if self.site.base_url.is_empty() {
            return Err(ConfigError::MissingField {
                field: "site.base_url",
            }
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_config_new() {
        let config = SiteConfig::new("My Site", "https://example.com");

        assert_eq!(config.site.name, "My Site");
        assert_eq!(config.site.base_url, "https://example.com");
        assert!(config.site.description.is_none());
        assert!(config.site.author.is_none());
    }

    #[test]
    fn test_build_config_defaults() {
        let config = BuildConfig::default();

        assert_eq!(config.content_dir, PathBuf::from("content"));
        assert_eq!(config.output_dir, PathBuf::from("dist"));
        assert_eq!(config.static_dir, PathBuf::from("static"));
        assert_eq!(config.styles_dir, PathBuf::from("styles"));
        assert_eq!(config.templates_dir, PathBuf::from("templates"));
    }

    #[test]
    fn test_site_config_validate_valid() {
        let config = SiteConfig::new("Test", "https://test.com");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_site_config_validate_empty_name() {
        let config = SiteConfig::new("", "https://test.com");
        let result = config.validate();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            GeneratorError::Config(inner) if matches!(*inner, ConfigError::MissingField { .. })
        ));
    }

    #[test]
    fn test_site_config_validate_empty_base_url() {
        let config = SiteConfig::new("Test", "");
        let result = config.validate();

        assert!(result.is_err());
    }

    #[test]
    fn test_site_config_from_str() {
        let toml = r#"
[site]
name = "Test Site"
base_url = "https://test.example.com"
description = "A test site"
author = "Test Author"

[build]
content_dir = "pages"
output_dir = "public"
"#;

        let config: SiteConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.site.name, "Test Site");
        assert_eq!(config.site.base_url, "https://test.example.com");
        assert_eq!(config.site.description, Some("A test site".to_string()));
        assert_eq!(config.site.author, Some("Test Author".to_string()));
        assert_eq!(config.build.content_dir, PathBuf::from("pages"));
        assert_eq!(config.build.output_dir, PathBuf::from("public"));
    }

    #[test]
    fn test_site_config_from_str_minimal() {
        let toml = r#"
[site]
name = "Minimal"
base_url = "https://minimal.com"
"#;

        let config: SiteConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.site.name, "Minimal");
        // Build config should use defaults
        assert_eq!(config.build.content_dir, PathBuf::from("content"));
    }

    #[test]
    fn test_site_config_from_str_missing_site() {
        let toml = r#"
[build]
content_dir = "content"
"#;

        let result: std::result::Result<SiteConfig, toml::de::Error> = toml::from_str(toml);
        assert!(result.is_err());
    }
}

#[test]
fn test_highlight_config_defaults() {
    let config = HighlightConfig::default();
    assert!(config.enabled);
    assert_eq!(config.class_prefix, "hl-");
}

#[test]
fn test_highlight_config_from_toml() {
    let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"

[highlight]
enabled = false
class_prefix = "syntax-"
"#;

    let config: SiteConfig = toml::from_str(toml).unwrap();
    assert!(!config.highlight.enabled);
    assert_eq!(config.highlight.class_prefix, "syntax-");
}

#[test]
fn test_highlight_config_missing_uses_defaults() {
    let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"
"#;

    let config: SiteConfig = toml::from_str(toml).unwrap();
    assert!(config.highlight.enabled);
    assert_eq!(config.highlight.class_prefix, "hl-");
}
