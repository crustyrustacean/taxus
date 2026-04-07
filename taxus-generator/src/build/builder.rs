// taxus-generator/src/build/builder.rs

//! Site builder for orchestrating the build pipeline.
//!
//! This module provides the main [`SiteBuilder`] type for building static sites.

use crate::build;
use crate::build::pipeline::{self, alias::AliasPage};
use crate::build::report::BuildReport;
use crate::config::SiteConfig;
use crate::error::{GeneratorError, Result};
use crate::highlighting::{CodeHighlighter, LanguageRegistry};
use crate::templates::SiteContext;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info, info_span};

/// Builder for generating static sites.
///
/// # Example
///
/// ```no_run
/// use taxus_lib::build::SiteBuilder;
/// use std::path::Path;
///
/// // Build from a directory containing site.toml
/// let report = SiteBuilder::from_dir(Path::new("."))?
///     .verbose(true)
///     .build()?;
///
/// report.print_summary();
/// # Ok::<(), taxus_lib::error::GeneratorError>(())
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
    /// 4. Copy co-located assets
    /// 5. Render pages with templates
    /// 6. Generate robots.txt
    /// 7. Generate sitemap.xml
    /// 8. Build and render taxonomy pages
    /// 9. Generate feeds (RSS/Atom)
    /// 10. Process assets (SCSS, static files)
    /// 11. Write output files
    ///
    /// # Errors
    ///
    /// Returns an error if any stage of the build fails.
    pub fn build(self) -> Result<BuildReport> {
        let span = info_span!("build", site = %self.config.site.name);
        let _enter = span.enter();

        let start = Instant::now();
        let output_dir = self.config.build.output_dir.clone();

        info!(
            site = %self.config.site.name,
            content_dir = %self.config.build.content_dir.display(),
            output_dir = %output_dir.display(),
            "Building site"
        );

        // Stage 1: Discover routes
        let _routes_span = info_span!("discover_routes").entered();
        info!("[1/12] Discovering routes...");
        let registry = pipeline::discover_routes(&self.config)?;

        if registry.is_empty() {
            return Err(GeneratorError::NoContent);
        }

        debug!("Found {} routes", registry.len());
        drop(_routes_span);

        // Stage 2: Load templates
        let _templates_span = info_span!("load_templates").entered();
        info!("[2/12] Loading templates...");
        let templates = pipeline::load_templates(&self.config)?;

        debug!(
            templates_dir = %self.config.build.templates_dir.display(),
            "Templates loaded"
        );
        drop(_templates_span);

        // create a code syntax highlighter
        let mut highlighter = if self.config.highlight.enabled {
            Some(CodeHighlighter::new(
                LanguageRegistry::new(),
                &self.config.highlight.class_prefix,
            ))
        } else {
            None
        };

        // Stage 3: Process content
        let _content_span = info_span!("process_content").entered();
        info!("[3/12] Processing content...");
        let processed = pipeline::process_content(
            &registry,
            &self.config,
            self.include_drafts,
            highlighter.as_mut(),
        )?;

        if processed.is_empty() {
            return Err(GeneratorError::NoContent);
        }

        // Count drafts skipped
        let total_routes = registry.len();
        let drafts_skipped = if self.include_drafts {
            0
        } else {
            total_routes - processed.len()
        };

        debug!(
            pages = processed.len(),
            drafts_skipped = drafts_skipped,
            "Content processed"
        );
        drop(_content_span);

        // Stage 4: Copy co-located assets
        let _colocated_span = info_span!("copy_colocated_assets").entered();
        info!("[4/12] Copying co-located assets...");
        let colocated_assets = pipeline::copy_colocated_assets(
            &self.config.build.content_dir,
            &output_dir,
            self.dry_run,
        )?;

        debug!(
            files_copied = colocated_assets.files_processed,
            "Co-located assets copied"
        );
        drop(_colocated_span);

        // Stage 5: Render pages
        let _render_span = info_span!("render_pages").entered();
        info!("[5/12] Rendering pages...");
        let site_context = SiteContext {
            name: self.config.site.name.clone(),
            base_url: self.config.site.base_url.clone(),
            description: self.config.site.description.clone(),
            author: self.config.site.author.clone(),
        };

        let rendered =
            pipeline::pages::render_pages(&processed, &templates, &site_context, self.verbose)?;
        drop(_render_span);

        // Stage 6: Generate robots.txt
        let _robots_span = info_span!("generate_robots").entered();
        info!("[6/12] Generating robots.txt...");
        let robots = pipeline::robots::generate_robots(&self.config)?;
        if let Some(ref robots) = robots {
            pipeline::robots::write_robots(robots, &output_dir, self.dry_run)?;
        }
        drop(_robots_span);

        // Stage 7: Generate sitemap.xml
        let _sitemap_span = info_span!("generate_sitemap").entered();
        info!("[7/12] Generating sitemap.xml...");
        let sitemap = pipeline::sitemap::generate_sitemap(&processed, &self.config)?;
        debug!(urls = sitemap.url_count, "Sitemap generated");
        pipeline::sitemap::write_sitemap(&sitemap, &output_dir, self.dry_run)?;
        drop(_sitemap_span);

        // Stage 8: Generate 404.html
        let _404_span = info_span!("generate_404").entered();
        info!("[8/12] Generating 404.html...");
        if let Some(ref page_404) = pipeline::not_found::generate_404(&templates, &site_context)? {
            pipeline::not_found::write_404(page_404, &output_dir, self.dry_run)?;
        }
        drop(_404_span);

        // Stage 9: Build and render taxonomy pages
        let _taxonomy_span = info_span!("render_taxonomy").entered();
        info!("[9/12] Building taxonomy pages...");
        let taxonomy_map = pipeline::taxonomy::build_taxonomy_map(&processed);
        let taxonomy_pages = pipeline::taxonomy::render_taxonomy_pages(
            &processed,
            &taxonomy_map,
            &templates,
            &site_context,
        )?;
        debug!(
            taxonomy_pages = taxonomy_pages.len(),
            "Taxonomy pages rendered"
        );
        drop(_taxonomy_span);

        // Stage 10: Generate feeds
        let _feeds_span = info_span!("generate_feeds").entered();
        info!("[10/12] Generating feeds...");
        let feeds = build::pipeline::feeds::generate_feeds(&processed, &self.config)?;
        debug!(feeds = feeds.len(), "Feeds generated");
        drop(_feeds_span);

        // Stage 11: Process assets
        let _assets_span = info_span!("process_assets").entered();
        info!("[11/12] Processing assets...");
        let mut assets = pipeline::process_assets(&self.config, &output_dir)?;

        // Merge co-located assets report into main assets report
        assets.merge(colocated_assets);

        debug!(files_processed = assets.files_processed, "Assets processed");
        drop(_assets_span);

        // Stage 12: Write output
        let _write_span = info_span!("write_output").entered();
        info!("[12/12] Writing output...");
        pipeline::write_output(&rendered, &output_dir, self.dry_run, self.verbose)?;

        // Write taxonomy pages
        if !taxonomy_pages.is_empty() {
            pipeline::taxonomy::write_taxonomy_pages(&taxonomy_pages, &output_dir, self.dry_run)?;
        }

        // Write feeds
        if !feeds.is_empty() {
            build::pipeline::feeds::write_feeds(&feeds, &output_dir, self.dry_run)?;
        }

        // Collect and write aliases
        let aliases: Vec<AliasPage> = processed
            .iter()
            .filter_map(|p| {
                let target_path = if p.page.frontmatter.slug.is_some() {
                    p.page.url_path()
                } else {
                    p.route.path.clone()
                };

                if p.page.aliases().is_empty() {
                    None
                } else {
                    Some(
                        p.page
                            .aliases()
                            .iter()
                            .map(move |alias| AliasPage::new(alias.clone(), target_path.clone())),
                    )
                }
            })
            .flatten()
            .collect();

        if !aliases.is_empty() {
            pipeline::alias::write_aliases(&aliases, &output_dir, self.dry_run)?;
            debug!(alias_count = aliases.len(), "Written alias redirects");
        }

        drop(_write_span);

        // Build the report
        let mut report = BuildReport::new(output_dir);
        report.pages_rendered = rendered.iter().filter(|r| r.route.is_page()).count();
        report.sections_rendered = rendered.iter().filter(|r| r.route.is_section()).count();
        report.drafts_skipped = drafts_skipped;
        report.sitemap_urls = sitemap.url_count;
        report.assets = assets;
        report.duration = start.elapsed();

        info!(
            duration_ms = report.duration.as_millis() as u64,
            pages = report.pages_rendered,
            sections = report.sections_rendered,
            "Build completed"
        );

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
    use crate::config::{BuildConfig, FeedConfig, HighlightConfig, SiteMeta};
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
            feed: FeedConfig::default(),
            highlight: HighlightConfig::default(),
            base_dir: PathBuf::new(),
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
