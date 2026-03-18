//! Site builder for orchestrating the build pipeline.
//!
//! This module provides the main [`SiteBuilder`] type for building static sites.

use crate::build::pipeline;
use crate::build::report::BuildReport;
use crate::config::SiteConfig;
use crate::error::{BuildError, Result};
use crate::templates::SiteContext;
use std::path::Path;
use std::time::Instant;

/// Builder for generating static sites.
///
/// # Example
///
/// ```no_run
/// use generator::build::SiteBuilder;
/// use std::path::Path;
///
/// // Build from a directory containing site.toml
/// let report = SiteBuilder::from_dir(Path::new("."))?
///     .verbose(true)
///     .build()?;
///
/// report.print_summary();
/// # Ok::<(), generator::error::GeneratorError>(())
/// ```
#[derive(Debug)]
pub struct SiteBuilder {
    /// Site configuration
    config: SiteConfig,
    /// Enable dry-run mode (no files written)
    dry_run: bool,
    /// Enable verbose output
    verbose: bool,
    /// Include draft pages in build
    include_drafts: bool,
}

impl SiteBuilder {
    /// Create a builder from a directory containing `site.toml`.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file cannot be found or parsed.
    pub fn from_dir(dir: &Path) -> Result<Self> {
        let config = pipeline::load_config(dir)?;
        Ok(Self::new(config))
    }

    /// Create a builder from an existing [`SiteConfig`].
    pub fn new(config: SiteConfig) -> Self {
        Self {
            config,
            dry_run: false,
            verbose: false,
            include_drafts: false,
        }
    }

    /// Enable or disable dry-run mode.
    ///
    /// In dry-run mode, no files are written to disk.
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Enable or disable verbose output.
    ///
    /// In verbose mode, detailed progress information is printed.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Include or exclude draft pages in the build.
    ///
    /// By default, drafts are excluded.
    pub fn include_drafts(mut self, include: bool) -> Self {
        self.include_drafts = include;
        self
    }

    /// Build the complete site.
    ///
    /// This orchestrates the full build pipeline:
    /// 1. Discover routes from content directory
    /// 2. Load templates
    /// 3. Process content files
    /// 4. Render pages with templates
    /// 5. Process assets (SCSS, static files)
    /// 6. Write output files
    ///
    /// # Errors
    ///
    /// Returns an error if any stage of the build fails.
    pub fn build(self) -> Result<BuildReport> {
        let start = Instant::now();
        let output_dir = self.config.build.output_dir.clone();

        if self.verbose {
            println!("Building site: {}", self.config.site.name);
            println!(
                "Content directory: {}",
                self.config.build.content_dir.display()
            );
            println!("Output directory: {}", output_dir.display());
        }

        // Stage 1: Discover routes
        if self.verbose {
            println!("\n[1/6] Discovering routes...");
        }
        let registry = pipeline::discover_routes(&self.config)?;

        if registry.is_empty() {
            return Err(BuildError::NoContent.into());
        }

        if self.verbose {
            println!("  Found {} routes", registry.len());
        }

        // Stage 2: Load templates
        if self.verbose {
            println!("\n[2/6] Loading templates...");
        }
        let templates = pipeline::load_templates(&self.config)?;

        if self.verbose {
            println!(
                "  Templates loaded from {}",
                self.config.build.templates_dir.display()
            );
        }

        // Stage 3: Process content
        if self.verbose {
            println!("\n[3/6] Processing content...");
        }
        let processed = pipeline::process_content(&registry, &self.config, self.include_drafts)?;

        if processed.is_empty() {
            return Err(BuildError::NoContent.into());
        }

        // Count drafts skipped
        let total_routes = registry.len();
        let drafts_skipped = if self.include_drafts {
            0
        } else {
            total_routes - processed.len()
        };

        if self.verbose {
            println!("  Processed {} pages", processed.len());
            if drafts_skipped > 0 {
                println!("  Skipped {} drafts", drafts_skipped);
            }
        }

        // Stage 4: Render pages
        if self.verbose {
            println!("\n[4/6] Rendering pages...");
        }
        let site_context = SiteContext {
            name: self.config.site.name.clone(),
            base_url: self.config.site.base_url.clone(),
            description: self.config.site.description.clone(),
            author: self.config.site.author.clone(),
        };

        let rendered = pipeline::render_pages(&processed, &templates, &site_context, self.verbose)?;

        // Stage 5: Process assets
        if self.verbose {
            println!("\n[5/6] Processing assets...");
        }
        let assets = pipeline::process_assets(&self.config, &output_dir)?;

        if self.verbose {
            println!("  Processed {} asset files", assets.files_processed);
        }

        // Stage 6: Write output
        if self.verbose {
            println!("\n[6/6] Writing output...");
        }
        pipeline::write_output(&rendered, &output_dir, self.dry_run, self.verbose)?;

        // Build the report
        let mut report = BuildReport::new(output_dir);
        report.pages_rendered = rendered.iter().filter(|r| r.route.is_page()).count();
        report.sections_rendered = rendered.iter().filter(|r| r.route.is_section()).count();
        report.drafts_skipped = drafts_skipped;
        report.assets = assets;
        report.duration = start.elapsed();

        if self.verbose {
            println!("\nBuild completed in {:.2}s", report.duration.as_secs_f64());
        }

        Ok(report)
    }

    /// Clean the output directory.
    ///
    /// Removes all files from the output directory.
    pub fn clean(self) -> Result<()> {
        pipeline::clean_output(&self.config.build.output_dir)
    }

    /// Get a reference to the site configuration.
    pub fn config(&self) -> &SiteConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BuildConfig, SiteMeta};
    use std::path::PathBuf;

    fn test_config() -> SiteConfig {
        SiteConfig {
            site: SiteMeta {
                name: "Test Site".to_string(),
                base_url: "https://example.com".to_string(),
                description: None,
                author: None,
            },
            build: BuildConfig {
                content_dir: PathBuf::from("tests/fixtures/content_site/content"),
                output_dir: PathBuf::from("tests/fixtures/content_site/dist"),
                static_dir: PathBuf::from("tests/fixtures/content_site/static"),
                styles_dir: PathBuf::from("tests/fixtures/content_site/styles"),
                templates_dir: PathBuf::from("tests/fixtures/template_site/templates"),
            },
        }
    }

    #[test]
    fn test_site_builder_new() {
        let config = test_config();
        let builder = SiteBuilder::new(config);
        assert!(!builder.dry_run);
        assert!(!builder.verbose);
        assert!(!builder.include_drafts);
    }

    #[test]
    fn test_site_builder_dry_run() {
        let config = test_config();
        let builder = SiteBuilder::new(config).dry_run(true);
        assert!(builder.dry_run);
    }

    #[test]
    fn test_site_builder_verbose() {
        let config = test_config();
        let builder = SiteBuilder::new(config).verbose(true);
        assert!(builder.verbose);
    }

    #[test]
    fn test_site_builder_include_drafts() {
        let config = test_config();
        let builder = SiteBuilder::new(config).include_drafts(true);
        assert!(builder.include_drafts);
    }

    #[test]
    fn test_site_builder_config() {
        let config = test_config();
        let builder = SiteBuilder::new(config);
        assert_eq!(builder.config().site.name, "Test Site");
    }

    #[test]
    fn test_site_builder_builder_chain() {
        let config = test_config();
        let builder = SiteBuilder::new(config)
            .dry_run(true)
            .verbose(true)
            .include_drafts(true);

        assert!(builder.dry_run);
        assert!(builder.verbose);
        assert!(builder.include_drafts);
    }
}
