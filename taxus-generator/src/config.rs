// taxus-generator/src/config.rs

//! Configuration types for the generator (parse phase).
//!
//! This module provides types for loading and representing site configuration
//! from `site.toml` files. The config is the second input to every
//! derivation after the Site Tree; see the book's
//! [Overview](https://crustyrustacean.github.io/taxus/theory/overview.html) chapter.

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
    /// Image processing configuration
    #[serde(default)]
    pub images: ImageConfig,
    /// Markdown rendering configuration
    #[serde(default)]
    pub markdown: MarkdownConfig,
    /// Base directory containing site.toml (not serialized)
    #[serde(skip)]
    pub base_dir: PathBuf,
}

/// Site metadata from the `[site]` section.
///
/// Unknown keys are rejected (#46): a typo like `autor` silently
/// falling back to the default is worse than a loud failure.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Build configuration from the `[build]` section.
///
/// Unknown keys are rejected (#46): `ouput_dir` must fail loudly
/// rather than quietly leaving `output_dir` at its default.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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

    /// Compile and embed the WASM hydration client (stage 14).
    ///
    /// `taxus init --no-islands` scaffolds a site whose templates never
    /// call `island()` and writes `islands = false` here; the build then
    /// skips the several hundred KB of `dist/wasm/` the site would never
    /// load (#56). Default `true`.
    #[serde(default = "default_islands")]
    pub islands: bool,

    /// Build and write the client-side search index (stage 13).
    ///
    /// A site without a search box never fetches `search_index.bin`;
    /// with this `false` the build skips it (#56). Default `true`.
    #[serde(default = "default_search")]
    pub search: bool,
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
fn default_islands() -> bool {
    true
}
fn default_search() -> bool {
    true
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            content_dir: default_content_dir(),
            output_dir: default_output_dir(),
            static_dir: default_static_dir(),
            styles_dir: default_styles_dir(),
            templates_dir: default_templates_dir(),
            islands: default_islands(),
            search: default_search(),
        }
    }
}

/// Feed configuration from the `[feed]` section.
///
/// Unknown keys are rejected (#46).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedConfig {
    /// Enable RSS feed generation
    #[serde(default = "default_rss_enabled")]
    pub rss_enabled: bool,

    /// Enable Atom feed generation
    #[serde(default = "default_atom_enabled")]
    pub atom_enabled: bool,

    /// Maximum number of entries in the feed. Unset means no limit;
    /// `0` is rejected by [`SiteConfig::validate`] — it has no meaning
    /// under "unset = unlimited", and a reader wanting to suppress feeds
    /// should use `rss_enabled` / `atom_enabled`.
    #[serde(default)]
    pub limit: Option<usize>,

    /// Include full content in feeds (vs summaries)
    #[serde(default = "default_full_content")]
    pub full_content: bool,

    /// Custom feed title (defaults to site name)
    pub title: Option<String>,

    /// Custom RSS feed filename (defaults to `feed.xml`)
    pub rss_path: Option<String>,

    /// Custom Atom feed filename (defaults to `feed.atom`)
    pub atom_path: Option<String>,

    /// Sections whose pages the feeds syndicate, as content-relative
    /// paths (`["blog"]`). A page anywhere under a listed section is
    /// included. Empty (the default) means every section in the site.
    #[serde(default)]
    pub sections: Vec<String>,
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
            limit: None,
            full_content: default_full_content(),
            title: None,
            rss_path: None,
            atom_path: None,
            sections: Vec::new(),
        }
    }
}

/// Highlight configuration from the `[highlight]` section.
///
/// Unknown keys are rejected (#46).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Image processing configuration from the `[images]` section.
///
/// `quality` only affects lossy formats (`"jpeg"` and `"webp"`). PNG is
/// lossless and ignores it. WebP is lossy only when taxus is built with the
/// `webp-lossy` feature (on by default); without it WebP output is lossless
/// and `quality` is ignored with a warning at build time.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageConfig {
    /// Responsive width breakpoints for generated variants
    #[serde(default = "default_image_widths")]
    pub widths: Vec<u32>,

    /// Output quality (1-100). Applies to `"jpeg"` and `"webp"` only;
    /// `"png"` ignores it. Validated by [`SiteConfig::validate`].
    #[serde(default = "default_image_quality")]
    pub quality: u8,

    /// Output format: `"webp"`, `"jpeg"` (alias `"jpg"`), or `"png"`.
    /// `"jpg"` is normalised to `"jpeg"` when loaded from a file.
    #[serde(default = "default_image_format")]
    pub format: String,

    /// Subdirectory within dist/ for processed images
    #[serde(default = "default_image_output_dir")]
    pub output_dir: PathBuf,
}

/// Markdown rendering configuration from the `[markdown]` section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkdownConfig {
    /// Insert a visible anchor link (`#`) into each heading for
    /// deep-linking. Defaults to false.
    #[serde(default)]
    pub insert_anchor_links: bool,
}

fn default_image_widths() -> Vec<u32> {
    vec![400, 800, 1200]
}
fn default_image_quality() -> u8 {
    80
}
fn default_image_format() -> String {
    "webp".to_string()
}
fn default_image_output_dir() -> PathBuf {
    PathBuf::from("images")
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            widths: default_image_widths(),
            quality: default_image_quality(),
            format: default_image_format(),
            output_dir: default_image_output_dir(),
        }
    }
}

impl ImageConfig {
    /// Accepted values for `format`, after normalisation.
    const FORMATS: [&str; 3] = ["webp", "jpeg", "png"];

    /// Canonicalise aliases: `"jpg"` becomes `"jpeg"`.
    pub fn normalize(&mut self) {
        if self.format == "jpg" {
            self.format = "jpeg".to_string();
        }
    }

    /// Validate `quality` (1..=100), `format` (`webp`, `jpeg`/`jpg`, `png`)
    /// and `widths` (non-empty; every entry is a `u32` by construction).
    pub fn validate(&self) -> Result<()> {
        if self.widths.is_empty() {
            return Err(ConfigError::Invalid(
                "images.widths must list at least one breakpoint; empty disables every \
                 variant. Remove the key to use the defaults [400, 800, 1200]."
                    .to_string(),
            )
            .into());
        }

        if !(1..=100).contains(&self.quality) {
            return Err(ConfigError::Invalid(format!(
                "images.quality must be between 1 and 100, got {}",
                self.quality
            ))
            .into());
        }

        let format = if self.format == "jpg" {
            "jpeg"
        } else {
            self.format.as_str()
        };
        if !Self::FORMATS.contains(&format) {
            return Err(ConfigError::Invalid(format!(
                "images.format must be one of \"webp\", \"jpeg\", \"jpg\", or \"png\", got \"{}\"",
                self.format
            ))
            .into());
        }

        Ok(())
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

        config.images.normalize();
        config.validate()?;

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
            images: ImageConfig::default(),
            markdown: MarkdownConfig::default(),
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

        if !self.site.base_url.starts_with("http://") && !self.site.base_url.starts_with("https://")
        {
            return Err(ConfigError::Invalid(format!(
                "site.base_url must start with http:// or https:// (taxus init already                  enforces this); got \"{}\"",
                self.site.base_url
            ))
            .into());
        }

        if self.feed.limit == Some(0) {
            return Err(ConfigError::Invalid(
                "[feed] limit = 0 has no meaning: unset means no limit. \
                 To disable feeds, set rss_enabled = false and/or \
                 atom_enabled = false instead."
                    .to_string(),
            )
            .into());
        }

        self.images.validate()?;

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
    fn test_feed_config_sections() {
        let toml = r#"
[site]
name = "Test Site"
base_url = "https://test.example.com"

[feed]
sections = ["blog", "notes/2026"]
"#;
        let config: SiteConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.feed.sections, ["blog", "notes/2026"]);

        let minimal: SiteConfig = toml::from_str(
            r#"
[site]
name = "Test Site"
base_url = "https://test.example.com"
"#,
        )
        .unwrap();
        assert!(minimal.feed.sections.is_empty());
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

    #[test]
    fn test_image_config_defaults() {
        let config = ImageConfig::default();
        assert_eq!(config.widths, vec![400, 800, 1200]);
        assert_eq!(config.quality, 80);
        assert_eq!(config.format, "webp");
        assert_eq!(config.output_dir, PathBuf::from("images"));
    }

    #[test]
    fn test_image_config_from_toml() {
        let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"

[images]
widths = [300, 600, 900]
quality = 75
format = "jpeg"
output_dir = "img"
"#;

        let config: SiteConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.images.widths, vec![300, 600, 900]);
        assert_eq!(config.images.quality, 75);
        assert_eq!(config.images.format, "jpeg");
        assert_eq!(config.images.output_dir, PathBuf::from("img"));
    }

    #[test]
    fn test_image_config_missing_uses_defaults() {
        let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"
"#;

        let config: SiteConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.images.widths, vec![400, 800, 1200]);
        assert_eq!(config.images.quality, 80);
        assert_eq!(config.images.format, "webp");
    }

    #[test]
    fn test_image_config_validate_rejects_quality_out_of_range() {
        for quality in [0u8, 101] {
            let mut config = SiteConfig::new("Test", "https://example.com");
            config.images.quality = quality;
            let err = config.validate().unwrap_err();
            assert!(
                matches!(&err, GeneratorError::Config(inner)
                if matches!(**inner, ConfigError::Invalid(ref msg) if msg.contains("images.quality"))),
                "quality {quality} should be rejected naming images.quality, got: {err}"
            );
        }
        for quality in [1u8, 80, 100] {
            let mut config = SiteConfig::new("Test", "https://example.com");
            config.images.quality = quality;
            assert!(
                config.validate().is_ok(),
                "quality {quality} should be valid"
            );
        }
    }

    #[test]
    fn test_image_config_validate_rejects_unknown_format() {
        let mut config = SiteConfig::new("Test", "https://example.com");
        config.images.format = "gif".to_string();
        let err = config.validate().unwrap_err();
        assert!(
            matches!(&err, GeneratorError::Config(inner)
            if matches!(**inner, ConfigError::Invalid(ref msg) if msg.contains("images.format"))),
            "format gif should be rejected naming images.format, got: {err}"
        );

        for format in ["webp", "jpeg", "jpg", "png"] {
            let mut config = SiteConfig::new("Test", "https://example.com");
            config.images.format = format.to_string();
            assert!(config.validate().is_ok(), "format {format} should be valid");
        }
    }

    #[test]
    fn test_image_config_from_file_normalises_jpg_and_validates() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("site.toml");

        std::fs::write(
            &path,
            r#"
[site]
name = "Test"
base_url = "https://example.com"

[images]
format = "jpg"
"#,
        )
        .unwrap();
        let config = SiteConfig::from_file(&path).unwrap();
        assert_eq!(config.images.format, "jpeg", "jpg should normalise to jpeg");

        std::fs::write(
            &path,
            r#"
[site]
name = "Test"
base_url = "https://example.com"

[images]
quality = 0
"#,
        )
        .unwrap();
        let err = SiteConfig::from_file(&path).unwrap_err();
        assert!(err.to_string().contains("images.quality"), "got: {err}");

        std::fs::write(
            &path,
            r#"
[site]
name = "Test"
base_url = "https://example.com"

[images]
format = "gif"
"#,
        )
        .unwrap();
        let err = SiteConfig::from_file(&path).unwrap_err();
        assert!(err.to_string().contains("images.format"), "got: {err}");
    }

    // ------------------------------------------------------------------
    // #46: unknown keys must fail loudly, per section
    // ------------------------------------------------------------------

    #[test]
    fn test_unknown_key_in_build_rejected() {
        let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"

[build]
ouput_dir = "public"
"#;
        let err = toml::from_str::<SiteConfig>(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ouput_dir") && msg.contains("unknown field"),
            "got: {msg}"
        );
        // The expected-field list proves the section: only [build] has content_dir.
        assert!(msg.contains("content_dir"), "got: {msg}");
    }

    #[test]
    fn test_unknown_key_in_images_rejected() {
        let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"

[images]
qality = 50
"#;
        let err = toml::from_str::<SiteConfig>(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("qality") && msg.contains("unknown field"),
            "got: {msg}"
        );
        assert!(msg.contains("widths"), "got: {msg}");
    }

    #[test]
    fn test_unknown_key_in_feed_rejected() {
        let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"

[feed]
limmit = 10
"#;
        let err = toml::from_str::<SiteConfig>(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("limmit") && msg.contains("unknown field"),
            "got: {msg}"
        );
        assert!(msg.contains("rss_enabled"), "got: {msg}");
    }

    #[test]
    fn test_unknown_key_in_highlight_rejected() {
        let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"

[highlight]
classprefix = "x-"
"#;
        let err = toml::from_str::<SiteConfig>(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("classprefix") && msg.contains("unknown field"),
            "got: {msg}"
        );
        assert!(msg.contains("enabled"), "got: {msg}");
    }

    #[test]
    fn test_unknown_key_in_site_rejected() {
        let toml = r#"
[site]
name = "Test"
base_url = "https://example.com"
autor = "Someone"
"#;
        let err = toml::from_str::<SiteConfig>(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("autor") && msg.contains("unknown field"),
            "got: {msg}"
        );
        assert!(msg.contains("base_url"), "got: {msg}");
    }

    /// #46: every known key round-trips — guards against a deny'd struct
    /// locking out a real key via a field-name typo.
    #[test]
    fn test_every_known_key_accepted() {
        let toml = r#"
[site]
name = "Full"
base_url = "https://example.com"
description = "d"
author = "a"

[build]
content_dir = "c"
output_dir = "o"
static_dir = "s"
styles_dir = "st"
templates_dir = "t"
islands = true
search = true

[feed]
rss_enabled = true
atom_enabled = true
limit = 15
full_content = false
title = "Feed"
rss_path = "feed.xml"
atom_path = "feed.atom"
sections = ["blog"]

[highlight]
enabled = true
class_prefix = "hl-"

[images]
widths = [400]
quality = 80
format = "webp"
output_dir = "images"

[markdown]
insert_anchor_links = false
"#;
        let config: SiteConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.site.name, "Full");
        assert_eq!(config.feed.sections, vec!["blog".to_string()]);
        assert!(config.build.islands && config.build.search);
    }

    // ------------------------------------------------------------------
    // #46: base_url scheme validation
    // ------------------------------------------------------------------

    #[test]
    fn test_base_url_must_be_http_or_https() {
        for url in ["example.com", "ftp://example.com", "httpx://example.com"] {
            let config = SiteConfig::new("Test", url);
            let err = config.validate().unwrap_err();
            assert!(
                matches!(&err, GeneratorError::Config(inner)
                    if matches!(**inner, ConfigError::Invalid(ref msg) if msg.contains("base_url"))),
                "{url} should be rejected naming site.base_url, got: {err}"
            );
        }
        for url in ["https://example.com", "http://localhost:3000"] {
            let config = SiteConfig::new("Test", url);
            assert!(
                config.validate().is_ok(),
                "{url} should be valid, got: {:?}",
                config.validate()
            );
        }
    }
}
