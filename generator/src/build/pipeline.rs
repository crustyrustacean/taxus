//! Build pipeline stages.
//!
//! This module provides the individual stages of the build pipeline.

use crate::assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
use crate::config::SiteConfig;
use crate::content::{Page, TaxonomyKind, TaxonomyMap};
use crate::error::{BuildError, GeneratorError, Result};
use crate::routes::{RouteDiscovery, RouteInfo, RouteRegistry};
use crate::templates::{
    PageContext, SiteContext, TaxonomyListContext, TaxonomyTermContext, TemplateContext,
    TemplateRenderer, TeraRenderer, compute_permalink,
};
use pulldown_cmark::{Parser, html::push_html};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span, info};

// Island-specific imports — only compiled when the `islands` feature is enabled.
#[cfg(feature = "islands")]
use common::components::counter::{Counter, CounterProps};
#[cfg(feature = "islands")]
use yew::ServerRenderer;

/// Processed page ready for rendering.
#[derive(Debug, Clone)]
pub struct ProcessedPage {
    /// Route information
    pub route: RouteInfo,
    /// Parsed page with frontmatter
    pub page: Page,
    /// Rendered HTML content
    pub html_content: String,
}

/// Rendered page ready for writing.
#[derive(Debug, Clone)]
pub struct RenderedPage {
    /// Route information
    pub route: RouteInfo,
    /// Final HTML content
    pub content: String,
}

/// Load configuration from a directory.
pub fn load_config(dir: &Path) -> std::result::Result<SiteConfig, BuildError> {
    SiteConfig::from_dir(dir).map_err(|e| match e {
        GeneratorError::Config(err) => BuildError::Config(err),
        GeneratorError::Io { path, source } => BuildError::Io { path, source },
        other => BuildError::Config(crate::error::ConfigError::Invalid(other.to_string())),
    })
}

/// Discover routes from the content directory.
pub fn discover_routes(config: &SiteConfig) -> std::result::Result<RouteRegistry, BuildError> {
    let discovery = RouteDiscovery::new(&config.build.content_dir);
    discovery.discover().map_err(BuildError::from)
}

/// Load templates from the templates directory.
pub fn load_templates(config: &SiteConfig) -> std::result::Result<TeraRenderer, BuildError> {
    TeraRenderer::from_dir(&config.build.templates_dir).map_err(BuildError::from)
}

/// Process content files into rendered HTML.
pub fn process_content(
    registry: &RouteRegistry,
    config: &SiteConfig,
    include_drafts: bool,
) -> Result<Vec<ProcessedPage>> {
    let mut pages = Vec::new();

    for route in registry.iter() {
        // Load the page from file
        let full_path = config.build.content_dir.join(&route.content_file);
        let page = Page::from_file(&full_path)?;

        // Skip drafts unless explicitly included
        if page.is_draft() && !include_drafts {
            continue;
        }

        // Resolve internal links in the content
        let resolved_content =
            resolve_internal_links(&page.raw_content, &route.content_file, registry)?;

        // Convert markdown to HTML
        let html_content = markdown_to_html(&resolved_content);

        pages.push(ProcessedPage {
            route: route.clone(),
            page,
            html_content,
        });
    }

    Ok(pages)
}

/// Render pages using templates.
pub fn render_pages(
    processed: &[ProcessedPage],
    templates: &TeraRenderer,
    site_context: &SiteContext,
    _verbose: bool,
) -> Result<Vec<RenderedPage>> {
    let span = debug_span!("render_pages", pages = processed.len());
    let _enter = span.enter();

    let mut rendered = Vec::new();

    for processed_page in processed {
        let template_name = processed_page.page.template();

        // Use custom slug if defined, otherwise use the discovered route path
        let url_path = if processed_page.page.frontmatter.slug.is_some() {
            processed_page.page.url_path()
        } else {
            processed_page.route.path.clone()
        };

        debug!(
            path = %url_path,
            template = %template_name,
            "Rendering page"
        );

        // Build the template context
        let permalink = compute_permalink(&site_context.base_url, &url_path);
        let page_context = PageContext {
            title: processed_page.page.frontmatter.title.clone(),
            description: processed_page.page.frontmatter.description.clone(),
            path: url_path.clone(),
            permalink,
            content: processed_page.html_content.clone(),
            raw_content: processed_page.page.raw_content.clone(),
            date: processed_page.page.frontmatter.date.map(|d| d.to_string()),
            draft: processed_page.page.is_draft(),
            summary: processed_page.page.summary(),
            word_count: processed_page.page.word_count(),
            reading_time: processed_page.page.reading_time(),
            tags: processed_page.page.tags().to_vec(),
            categories: processed_page.page.categories().to_vec(),
            series: processed_page.page.series().map(|s| s.to_string()),
        };

        let context = TemplateContext::new(site_context.clone()).with_page(page_context);

        // Render the template
        let content = templates.render(template_name, &context).map_err(|e| {
            BuildError::PageRenderFailed {
                path: url_path.clone(),
                source: e,
            }
        })?;

        // Create route with potentially overridden path
        let output_file = if processed_page.page.frontmatter.slug.is_some() {
            let slug = processed_page.page.slug();
            if slug == "_index" {
                std::path::PathBuf::from("index.html")
            } else {
                std::path::PathBuf::from(slug).join("index.html")
            }
        } else {
            processed_page.route.output_file.clone()
        };

        let route = RouteInfo::new(
            url_path,
            processed_page.route.content_file.clone(),
            output_file,
            processed_page.route.kind,
        )
        .map_err(BuildError::from)?;

        rendered.push(RenderedPage { route, content });
    }

    info!("Rendered {} pages", rendered.len());
    Ok(rendered)
}

/// Build taxonomy map from processed pages.
pub fn build_taxonomy_map(processed: &[ProcessedPage]) -> TaxonomyMap {
    // Extract pages from processed pages
    let pages: Vec<&Page> = processed.iter().map(|p| &p.page).collect();
    // Convert to owned pages for from_pages
    let owned_pages: Vec<Page> = pages.iter().map(|&p| p.clone()).collect();
    TaxonomyMap::from_pages(&owned_pages)
}

/// Rendered taxonomy page.
#[derive(Debug, Clone)]
pub struct RenderedTaxonomy {
    /// URL path for the taxonomy page
    pub path: String,
    /// Output file path
    pub output_file: PathBuf,
    /// Rendered HTML content
    pub content: String,
}

/// Render taxonomy pages (list pages and term pages).
///
/// This generates:
/// - Taxonomy list pages (e.g., /tags/, /categories/, /series/)
/// - Taxonomy term pages (e.g., /tags/rust/, /categories/tutorial/)
pub fn render_taxonomy_pages(
    processed: &[ProcessedPage],
    taxonomy_map: &TaxonomyMap,
    templates: &TeraRenderer,
    site_context: &SiteContext,
) -> Result<Vec<RenderedTaxonomy>> {
    let mut rendered = Vec::new();

    // Build a lookup from content file path (as string) to processed page for page context creation
    let page_lookup: std::collections::HashMap<String, &ProcessedPage> = processed
        .iter()
        .map(|p| (p.route.content_file.to_string_lossy().to_string(), p))
        .collect();

    // Process each taxonomy kind
    for kind in [
        TaxonomyKind::Tag,
        TaxonomyKind::Category,
        TaxonomyKind::Series,
    ] {
        let terms: Vec<_> = match kind {
            TaxonomyKind::Tag => taxonomy_map.tags(),
            TaxonomyKind::Category => taxonomy_map.categories(),
            TaxonomyKind::Series => taxonomy_map.series(),
        };

        if terms.is_empty() {
            continue;
        }

        let kind_name = kind.plural_name();
        let list_path = format!("/{}/", kind.path_prefix());

        // Render taxonomy list page
        let list_context = TaxonomyListContext {
            kind: kind_name.to_string(),
            path: list_path.clone(),
            terms: terms
                .iter()
                .map(|term| {
                    let term_path = term.url_path();
                    TaxonomyTermContext {
                        kind: kind_name.to_string(),
                        name: term.name.clone(),
                        slug: term.slug.clone(),
                        path: term_path,
                        page_count: term.page_count,
                        pages: vec![], // Empty for list page
                    }
                })
                .collect(),
        };

        let template_name = format!("{}.html", kind.path_prefix());

        // Only render if template exists
        if templates.has_template(&template_name) {
            let context = TemplateContext::new(site_context.clone());
            let context = context.with_extra(
                vec![(
                    "taxonomy".to_string(),
                    serde_json::to_value(&list_context).unwrap(),
                )]
                .into_iter()
                .collect(),
            );

            let content = templates.render(&template_name, &context).map_err(|e| {
                BuildError::PageRenderFailed {
                    path: list_path.clone(),
                    source: e,
                }
            })?;

            let output_file = PathBuf::from(kind.path_prefix()).join("index.html");

            rendered.push(RenderedTaxonomy {
                path: list_path,
                output_file,
                content,
            });
        }

        // Render individual term pages
        for term in terms {
            let term_path = term.url_path();
            let template_name = format!("{}_term.html", kind.path_prefix());

            // Only render if template exists
            if templates.has_template(&template_name) {
                // Build page contexts for pages with this term
                let mut page_contexts: Vec<PageContext> = Vec::new();
                for page_path in &term.page_paths {
                    if let Some(proc_page) = page_lookup.get(page_path) {
                        let url_path = if proc_page.page.frontmatter.slug.is_some() {
                            proc_page.page.url_path()
                        } else {
                            proc_page.route.path.clone()
                        };

                        let permalink = compute_permalink(&site_context.base_url, &url_path);
                        let page_context = PageContext {
                            title: proc_page.page.frontmatter.title.clone(),
                            description: proc_page.page.frontmatter.description.clone(),
                            path: url_path,
                            permalink,
                            content: proc_page.html_content.clone(),
                            raw_content: proc_page.page.raw_content.clone(),
                            date: proc_page.page.frontmatter.date.map(|d| d.to_string()),
                            draft: proc_page.page.is_draft(),
                            summary: proc_page.page.summary(),
                            word_count: proc_page.page.word_count(),
                            reading_time: proc_page.page.reading_time(),
                            tags: proc_page.page.tags().to_vec(),
                            categories: proc_page.page.categories().to_vec(),
                            series: proc_page.page.series().map(|s| s.to_string()),
                        };
                        page_contexts.push(page_context);
                    }
                }

                let term_context = TaxonomyTermContext {
                    kind: kind_name.to_string(),
                    name: term.name.clone(),
                    slug: term.slug.clone(),
                    path: term_path.clone(),
                    page_count: term.page_count,
                    pages: page_contexts,
                };

                let context = TemplateContext::new(site_context.clone());
                let context = context.with_extra(
                    vec![(
                        "taxonomy".to_string(),
                        serde_json::to_value(&term_context).unwrap(),
                    )]
                    .into_iter()
                    .collect(),
                );

                let content = templates.render(&template_name, &context).map_err(|e| {
                    BuildError::PageRenderFailed {
                        path: term_path.clone(),
                        source: e,
                    }
                })?;

                let output_file =
                    PathBuf::from(&term_path.trim_start_matches('/')).join("index.html");

                rendered.push(RenderedTaxonomy {
                    path: term_path,
                    output_file,
                    content,
                });
            }
        }
    }

    if !rendered.is_empty() {
        info!("Rendered {} taxonomy pages", rendered.len());
    }
    Ok(rendered)
}

/// Write taxonomy pages to output files.
pub fn write_taxonomy_pages(
    taxonomy_pages: &[RenderedTaxonomy],
    output_dir: &Path,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping taxonomy page writes");
        return Ok(());
    }

    for taxonomy_page in taxonomy_pages {
        let output_path = output_dir.join(&taxonomy_page.output_file);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| BuildError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write the file
        fs::write(&output_path, &taxonomy_page.content).map_err(|e| BuildError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        debug!(
            path = %output_path.display(),
            route = %taxonomy_page.path,
            "Written taxonomy page"
        );
    }

    if !taxonomy_pages.is_empty() {
        info!("Wrote {} taxonomy pages", taxonomy_pages.len());
    }
    Ok(())
}

/// Process assets (SCSS and static files).
pub fn process_assets(config: &SiteConfig, output_dir: &Path) -> Result<AssetReport> {
    let mut report = AssetReport::new();

    // Process SCSS files
    if config.build.styles_dir.exists() {
        let scss_processor = ScssProcessor::new();
        let css_output = output_dir.join("css");
        let scss_report = scss_processor
            .process(&config.build.styles_dir, &css_output)
            .map_err(BuildError::from)?;
        report.merge(scss_report);
    }

    // Copy static files
    if config.build.static_dir.exists() {
        let static_copier = StaticCopier::new();
        let static_output = output_dir.join("static");
        let static_report = static_copier
            .process(&config.build.static_dir, &static_output)
            .map_err(BuildError::from)?;
        report.merge(static_report);
    }

    Ok(report)
}

/// Copy co-located assets from content directory to output directory.
///
/// Walks the content directory and copies any non-.md files to the same
/// relative path in the output directory. This allows assets like images
/// to be co-located with their markdown files.
///
/// # Example
///
/// If content directory contains:
/// - `content/blog/post.md` → processed as HTML
/// - `content/blog/photo.jpg` → copied to `output/blog/photo.jpg`
///
/// # Arguments
///
/// * `content_dir` - Path to the content directory
/// * `output_dir` - Path to the output directory
/// * `dry_run` - If true, no files are written
///
/// # Returns
///
/// An `AssetReport` containing the number of files copied.
pub fn copy_colocated_assets(
    content_dir: &Path,
    output_dir: &Path,
    dry_run: bool,
) -> Result<AssetReport> {
    use walkdir::WalkDir;

    let mut report = AssetReport::new();

    // Skip if content directory doesn't exist
    if !content_dir.exists() {
        debug!(
            content_dir = %content_dir.display(),
            "Content directory does not exist, skipping co-located asset copy"
        );
        return Ok(report);
    }

    let span = debug_span!("copy_colocated_assets", content_dir = %content_dir.display());
    let _enter = span.enter();

    for entry in WalkDir::new(content_dir).into_iter().filter_map(|e| e.ok()) {
        // Skip directories
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Skip markdown files (they become HTML pages)
        if path.extension().is_some_and(|ext| ext == "md") {
            continue;
        }

        // Calculate relative path from content directory
        let relative = match path.strip_prefix(content_dir) {
            Ok(rel) => rel,
            Err(_) => continue, // Shouldn't happen, but skip if it does
        };

        // Construct destination path
        let dest_path = output_dir.join(relative);

        if dry_run {
            debug!(
                src = %path.display(),
                dest = %dest_path.display(),
                "Dry run - would copy co-located asset"
            );
            report.add_processed();
            continue;
        }

        // Create parent directories
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| BuildError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Copy the file
        fs::copy(path, &dest_path).map_err(|e| BuildError::Io {
            path: dest_path.clone(),
            source: e,
        })?;

        debug!(
            src = %path.display(),
            dest = %dest_path.display(),
            "Copied co-located asset"
        );

        report.add_processed();
    }

    if report.files_processed > 0 {
        info!("Copied {} co-located assets", report.files_processed);
    }

    Ok(report)
}

/// Generated feed file.
#[derive(Debug, Clone)]
pub struct GeneratedFeed {
    /// Feed filename (e.g., "feed.xml", "atom.xml")
    pub filename: String,
    /// Feed content (XML)
    pub content: String,
}

/// Generate RSS and Atom feeds from processed pages.
pub fn generate_feeds(
    processed: &[ProcessedPage],
    config: &SiteConfig,
) -> Result<Vec<GeneratedFeed>> {
    use crate::feed::{FeedConfig as FeedGenConfig, FeedGenerator};

    let mut feeds = Vec::new();

    // Skip if both feeds are disabled
    if !config.feed.rss_enabled && !config.feed.atom_enabled {
        return Ok(feeds);
    }

    // Collect pages for feed generation
    let pages: Vec<crate::content::Page> = processed
        .iter()
        .filter(|p| !p.page.is_draft()) // Exclude drafts from feeds
        .map(|p| {
            let mut page = p.page.clone();
            // Update the page path to use the correct URL path
            let url_path = if p.page.frontmatter.slug.is_some() {
                p.page.url_path()
            } else {
                p.route.path.clone()
            };
            page.path = url_path;
            // Set content for full-content feeds
            if config.feed.full_content {
                page.content = Some(p.html_content.clone());
            }
            page
        })
        .collect();

    // Build feed generator config
    let feed_gen_config = FeedGenConfig {
        title: config
            .feed
            .title
            .clone()
            .unwrap_or_else(|| config.site.name.clone()),
        base_url: config.site.base_url.clone(),
        description: config.site.description.clone().unwrap_or_default(),
        author: config.site.author.clone(),
        limit: if config.feed.limit > 0 {
            config.feed.limit
        } else {
            20
        },
        full_content: config.feed.full_content,
        ..Default::default()
    };

    let generator = FeedGenerator::new(feed_gen_config);

    // Generate RSS feed if enabled
    if config.feed.rss_enabled {
        let rss_content = generator.generate_rss(&pages)?;
        let filename = config
            .feed
            .rss_path
            .clone()
            .unwrap_or_else(|| generator.rss_filename());
        feeds.push(GeneratedFeed {
            filename,
            content: rss_content,
        });
        info!("Generated RSS feed");
    }

    // Generate Atom feed if enabled
    if config.feed.atom_enabled {
        let atom_content = generator.generate_atom(&pages)?;
        let filename = config
            .feed
            .atom_path
            .clone()
            .unwrap_or_else(|| generator.atom_filename());
        feeds.push(GeneratedFeed {
            filename,
            content: atom_content,
        });
        info!("Generated Atom feed");
    }

    Ok(feeds)
}

/// Write feed files to output directory.
pub fn write_feeds(feeds: &[GeneratedFeed], output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping feed writes");
        return Ok(());
    }

    for feed in feeds {
        let output_path = output_dir.join(&feed.filename);

        // Write the feed file
        fs::write(&output_path, &feed.content).map_err(|e| BuildError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        debug!(
            path = %output_path.display(),
            "Written feed file"
        );
    }

    if !feeds.is_empty() {
        info!("Wrote {} feed files", feeds.len());
    }

    Ok(())
}

/// Generated robots.txt content.
#[derive(Debug, Clone)]
pub struct GeneratedRobots {
    /// Robots.txt content
    pub content: String,
}

/// Generate robots.txt content.
///
/// If a robots.txt already exists in the static directory, returns None
/// (the existing file will be copied by StaticCopier). Otherwise, generates
/// a default robots.txt with sitemap reference.
pub fn generate_robots(config: &SiteConfig) -> Result<Option<GeneratedRobots>> {
    // Check if static/robots.txt already exists
    let static_robots = config.build.static_dir.join("robots.txt");
    if static_robots.exists() {
        debug!(
            path = %static_robots.display(),
            "Static robots.txt exists, skipping generation"
        );
        return Ok(None);
    }

    // Generate default robots.txt
    let base_url = &config.site.base_url;
    let content = format!(
        r#"User-agent: *
Allow: /

Sitemap: {}/sitemap.xml
"#,
        base_url.trim_end_matches('/')
    );

    info!("Generated default robots.txt");
    Ok(Some(GeneratedRobots { content }))
}

/// Write robots.txt to output directory.
pub fn write_robots(robots: &GeneratedRobots, output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping robots.txt write");
        return Ok(());
    }

    let output_path = output_dir.join("robots.txt");

    // Write the robots.txt file
    fs::write(&output_path, &robots.content).map_err(|e| BuildError::Io {
        path: output_path.clone(),
        source: e,
    })?;

    debug!(
        path = %output_path.display(),
        "Written robots.txt"
    );

    info!("Wrote robots.txt");
    Ok(())
}

/// Sitemap URL entry.
#[derive(Debug, Clone)]
pub struct SitemapUrl {
    /// Full URL (base_url + path)
    pub loc: String,
    /// Last modification date (YYYY-MM-DD format)
    pub lastmod: Option<String>,
    /// Change frequency
    pub changefreq: String,
    /// Priority (0.0 to 1.0)
    pub priority: String,
}

/// Generated sitemap.xml content.
#[derive(Debug, Clone)]
pub struct GeneratedSitemap {
    /// Sitemap XML content
    pub content: String,
    /// Number of URLs in the sitemap
    pub url_count: usize,
}

/// Generate sitemap.xml from processed pages.
///
/// Creates a sitemap with:
/// - All non-draft pages from the registry
/// - lastmod from page date if available
/// - Priority: 1.0 for home, 0.8 for sections, 0.7 for pages
/// - changefreq: weekly for home, monthly for others
pub fn generate_sitemap(
    processed: &[ProcessedPage],
    config: &SiteConfig,
) -> Result<GeneratedSitemap> {
    let base_url = config.site.base_url.trim_end_matches('/');
    let mut urls: Vec<SitemapUrl> = Vec::new();

    for processed_page in processed {
        // Skip drafts
        if processed_page.page.is_draft() {
            continue;
        }

        // Get the URL path (respecting custom slugs)
        let url_path = if processed_page.page.frontmatter.slug.is_some() {
            processed_page.page.url_path()
        } else {
            processed_page.route.path.clone()
        };

        // Build full URL using compute_permalink for proper slash handling
        let loc = compute_permalink(base_url, &url_path);

        // Get lastmod from page date
        let lastmod = processed_page
            .page
            .frontmatter
            .date
            .map(|d| d.format("%Y-%m-%d").to_string());

        // Determine priority and changefreq based on route type
        let (priority, changefreq) = if url_path == "/" {
            ("1.0".to_string(), "weekly".to_string())
        } else if processed_page.route.is_section() {
            ("0.8".to_string(), "monthly".to_string())
        } else {
            ("0.7".to_string(), "monthly".to_string())
        };

        urls.push(SitemapUrl {
            loc,
            lastmod,
            changefreq,
            priority,
        });
    }

    // Sort URLs by path for consistent output
    urls.sort_by(|a, b| a.loc.cmp(&b.loc));

    // Generate XML
    let mut xml = String::new();
    xml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
    );

    for url in &urls {
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", url.loc));
        if let Some(ref lastmod) = url.lastmod {
            xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
        }
        xml.push_str(&format!(
            "    <changefreq>{}</changefreq>\n",
            url.changefreq
        ));
        xml.push_str(&format!("    <priority>{}</priority>\n", url.priority));
        xml.push_str("  </url>\n");
    }

    xml.push_str("</urlset>\n");

    let url_count = urls.len();
    info!("Generated sitemap.xml with {} URLs", url_count);

    Ok(GeneratedSitemap {
        content: xml,
        url_count,
    })
}

/// Write sitemap.xml to output directory.
pub fn write_sitemap(sitemap: &GeneratedSitemap, output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping sitemap.xml write");
        return Ok(());
    }

    let output_path = output_dir.join("sitemap.xml");

    // Write the sitemap file
    fs::write(&output_path, &sitemap.content).map_err(|e| BuildError::Io {
        path: output_path.clone(),
        source: e,
    })?;

    debug!(
        path = %output_path.display(),
        urls = sitemap.url_count,
        "Written sitemap.xml"
    );

    info!("Wrote sitemap.xml with {} URLs", sitemap.url_count);
    Ok(())
}

/// Write rendered pages to output files.
pub fn write_output(
    rendered: &[RenderedPage],
    output_dir: &Path,
    dry_run: bool,
    _verbose: bool,
) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping file writes");
        return Ok(());
    }

    for rendered_page in rendered {
        let output_path = output_dir.join(&rendered_page.route.output_file);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| BuildError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write the file
        fs::write(&output_path, &rendered_page.content).map_err(|e| BuildError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        debug!(
            path = %output_path.display(),
            route = %rendered_page.route.path,
            "Written output file"
        );
    }

    info!("Wrote {} files", rendered.len());
    Ok(())
}

/// Alias page for redirects.
#[derive(Debug, Clone)]
pub struct AliasPage {
    /// Alias URL path (e.g., "/old-url/")
    pub alias_path: String,
    /// Target URL path (e.g., "/new-url/")
    pub target_path: String,
    /// Output file path for the redirect page
    pub output_file: PathBuf,
}

impl AliasPage {
    /// Create a new alias page.
    pub fn new(alias_path: String, target_path: String) -> Self {
        // Convert alias path to output file path
        // "/old-url/" -> "old-url/index.html"
        let output_file = if alias_path == "/" {
            PathBuf::from("index.html")
        } else {
            let trimmed = alias_path.trim_start_matches('/').trim_end_matches('/');
            if trimmed.is_empty() {
                PathBuf::from("index.html")
            } else {
                PathBuf::from(trimmed).join("index.html")
            }
        };

        Self {
            alias_path,
            target_path,
            output_file,
        }
    }

    /// Generate HTML redirect page.
    pub fn to_html(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0;url={}">
    <link rel="canonical" href="{}">
    <title>Redirecting...</title>
</head>
<body>
    <p>Redirecting to <a href="{}">{}</a>...</p>
</body>
</html>"#,
            self.target_path, self.target_path, self.target_path, self.target_path
        )
    }
}

/// Write alias redirect pages.
pub fn write_aliases(aliases: &[AliasPage], output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping alias file writes");
        return Ok(());
    }

    for alias in aliases {
        let output_path = output_dir.join(&alias.output_file);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| BuildError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write the redirect HTML
        let html = alias.to_html();
        fs::write(&output_path, &html).map_err(|e| BuildError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        debug!(
            path = %output_path.display(),
            alias = %alias.alias_path,
            target = %alias.target_path,
            "Written alias redirect file"
        );
    }

    info!("Wrote {} alias redirects", aliases.len());
    Ok(())
}

/// Clean the output directory.
pub fn clean_output(output_dir: &Path) -> Result<()> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir).map_err(|e| BuildError::Io {
            path: output_dir.to_path_buf(),
            source: e,
        })?;
    }
    Ok(())
}

/// Resolve internal links in content.
///
/// Internal links use the syntax `](@/path/to/file.md)` where the path is relative
/// to the content directory root. This function resolves them to the actual URL path.
///
/// # Errors
///
/// Returns a `BuildError::BrokenInternalLink` if any target path is not found in the registry.
pub fn resolve_internal_links(
    content: &str,
    source_file: &Path,
    registry: &RouteRegistry,
) -> std::result::Result<String, BuildError> {
    let mut result = String::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("](@/") {
        // Find the opening bracket to capture the link text
        let bracket_pos = remaining[..start].rfind('[');

        let Some(bracket_pos) = bracket_pos else {
            // No opening bracket found, append and continue
            result.push_str(&remaining[..start + 4]);
            remaining = &remaining[start + 4..];
            continue;
        };

        // Append content before the link
        result.push_str(&remaining[..bracket_pos]);

        // Extract link text
        let link_text = &remaining[bracket_pos + 1..start];

        // Find the closing parenthesis
        let after_at = start + 4; // Skip "](@/"
        let end_paren = remaining[after_at..].find(')').map(|p| after_at + p);

        let Some(end_paren) = end_paren else {
            // No closing paren found, append and continue
            result.push_str(&remaining[bracket_pos..start + 4]);
            remaining = &remaining[start + 4..];
            continue;
        };

        // Extract the target path
        let target_path = &remaining[after_at..end_paren];

        // Look up the target in the registry
        let target_pathbuf = PathBuf::from(target_path);
        let route = registry.find_by_content_file(&target_pathbuf);

        let Some(route) = route else {
            return Err(BuildError::BrokenInternalLink {
                file: source_file.display().to_string(),
                target: format!("@/{}", target_path),
            });
        };

        // Append the resolved link
        result.push_str(&format!("[{}]({})", link_text, route.path));

        // Move past this link
        remaining = &remaining[end_paren + 1..];
    }

    // Append any remaining content
    result.push_str(remaining);

    Ok(result)
}

/// Convert markdown content to HTML.
fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut output = String::new();
    push_html(&mut output, parser);
    output
}

/// SSR a Yew island component and wrap it in the hydration mount div.
///
/// Only compiled when the `islands` feature is enabled.
#[cfg(feature = "islands")]
pub fn render_island_counter(props: CounterProps) -> String {
    // Serialize props to JSON for the data attribute
    let props_json = serde_json::to_string(&props).unwrap_or_else(|_| "{}".to_string());

    // Build a self-contained single-threaded runtime for this SSR call.
    // We cannot use Handle::current() because the generator's main() is synchronous
    // and has no ambient tokio runtime running. Builder::new_current_thread() creates
    // a temporary runtime that exists only for the duration of block_on.
    let ssr_html = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("Failed to build tokio runtime for island SSR")
        .block_on(async {
            ServerRenderer::<Counter>::with_props(move || props)
                .render()
                .await
        });

    // Emit the mount point wrapper around the SSR output
    format!(r#"<div data-island="Counter" data-props='{props_json}'>{ssr_html}</div>"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html() {
        let markdown = "# Hello\n\nThis is **bold** text.";
        let html = markdown_to_html(markdown);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_markdown_to_html_empty() {
        let html = markdown_to_html("");
        assert!(html.is_empty());
    }

    #[test]
    fn test_markdown_to_html_links() {
        let markdown = "[link](https://example.com)";
        let html = markdown_to_html(markdown);
        assert!(html.contains("<a href=\"https://example.com\">link</a>"));
    }

    #[test]
    fn test_markdown_to_html_code() {
        let markdown = "```\ncode\n```";
        let html = markdown_to_html(markdown);
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code>"));
    }

    // ============================================
    // Phase 1.3: Slug Customization Tests
    // ============================================

    #[test]
    fn test_render_pages_with_custom_slug() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a page with custom slug
        let content = r#"
+++
title = "Test Post"
slug = "custom-url"
+++
This is the content.
"#;
        let page = Page::from_str(content.trim_start(), "original-filename.md").unwrap();

        // Create a processed page
        let route = RouteInfo::new(
            "/original-filename/".to_string(),
            std::path::PathBuf::from("original-filename.md"),
            std::path::PathBuf::from("original-filename/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let processed = ProcessedPage {
            route,
            page,
            html_content: "<p>This is the content.</p>".to_string(),
        };

        // Create a minimal template renderer
        let templates = TeraRenderer::from_dir(std::path::Path::new(
            "tests/fixtures/template_site/templates",
        ))
        .unwrap();

        let site_context = SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: None,
            author: None,
        };

        // Render the page
        let rendered = render_pages(&[processed], &templates, &site_context, false).unwrap();

        assert_eq!(rendered.len(), 1);
        // The URL path should use the custom slug
        assert_eq!(rendered[0].route.path, "/custom-url/");
        // The output file should use the custom slug
        assert_eq!(
            rendered[0].route.output_file,
            std::path::PathBuf::from("custom-url/index.html")
        );
    }

    #[test]
    fn test_render_pages_without_custom_slug() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a page without custom slug
        let content = r#"
+++
title = "Test Post"
+++
This is the content.
"#;
        let page = Page::from_str(content.trim_start(), "my-post.md").unwrap();

        // Create a processed page
        let route = RouteInfo::new(
            "/my-post/".to_string(),
            std::path::PathBuf::from("my-post.md"),
            std::path::PathBuf::from("my-post/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let processed = ProcessedPage {
            route,
            page,
            html_content: "<p>This is the content.</p>".to_string(),
        };

        // Create a minimal template renderer
        let templates = TeraRenderer::from_dir(std::path::Path::new(
            "tests/fixtures/template_site/templates",
        ))
        .unwrap();

        let site_context = SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: None,
            author: None,
        };

        // Render the page
        let rendered = render_pages(&[processed], &templates, &site_context, false).unwrap();

        assert_eq!(rendered.len(), 1);
        // The URL path should use the original route path
        assert_eq!(rendered[0].route.path, "/my-post/");
        // The output file should use the original path
        assert_eq!(
            rendered[0].route.output_file,
            std::path::PathBuf::from("my-post/index.html")
        );
    }

    #[test]
    fn test_alias_page_new() {
        let alias = AliasPage::new("/old-url/".to_string(), "/new-url/".to_string());
        assert_eq!(alias.alias_path, "/old-url/");
        assert_eq!(alias.target_path, "/new-url/");
        assert_eq!(alias.output_file, PathBuf::from("old-url/index.html"));
    }

    #[test]
    fn test_alias_page_root() {
        let alias = AliasPage::new("/".to_string(), "/new-home/".to_string());
        assert_eq!(alias.alias_path, "/");
        assert_eq!(alias.target_path, "/new-home/");
        assert_eq!(alias.output_file, PathBuf::from("index.html"));
    }

    #[test]
    fn test_alias_page_to_html() {
        let alias = AliasPage::new("/old-url/".to_string(), "/new-url/".to_string());
        let html = alias.to_html();

        // Check that the HTML contains the redirect elements
        assert!(html.contains(r#"http-equiv="refresh""#));
        assert!(html.contains("0;url=/new-url/"));
        assert!(html.contains(r#"rel="canonical""#));
        assert!(html.contains("href=\"/new-url/\""));
        assert!(html.contains("<a href=\"/new-url/\""));
    }

    #[test]
    fn test_alias_page_deep_path() {
        let alias = AliasPage::new("/blog/old-post/".to_string(), "/blog/new-post/".to_string());
        assert_eq!(alias.alias_path, "/blog/old-post/");
        assert_eq!(alias.target_path, "/blog/new-post/");
        assert_eq!(alias.output_file, PathBuf::from("blog/old-post/index.html"));
    }

    // ============================================
    // Phase 2.3: Taxonomy Page Tests
    // ============================================

    #[test]
    fn test_build_taxonomy_map() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create pages with taxonomies
        let content1 = r#"
+++
title = "Post 1"
tags = ["rust", "web"]
categories = ["tutorial"]
+++
Content 1
"#;
        let content2 = r#"
+++
title = "Post 2"
tags = ["rust"]
series = "Learning Rust"
+++
Content 2
"#;

        let page1 = Page::from_str(content1.trim_start(), "post-1.md").unwrap();
        let page2 = Page::from_str(content2.trim_start(), "post-2.md").unwrap();

        let route1 = RouteInfo::new(
            "/post-1/".to_string(),
            std::path::PathBuf::from("post-1.md"),
            std::path::PathBuf::from("post-1/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let route2 = RouteInfo::new(
            "/post-2/".to_string(),
            std::path::PathBuf::from("post-2.md"),
            std::path::PathBuf::from("post-2/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let processed = vec![
            ProcessedPage {
                route: route1,
                page: page1,
                html_content: "<p>Content 1</p>".to_string(),
            },
            ProcessedPage {
                route: route2,
                page: page2,
                html_content: "<p>Content 2</p>".to_string(),
            },
        ];

        let taxonomy_map = build_taxonomy_map(&processed);

        // Check tags
        assert_eq!(taxonomy_map.tags().len(), 2);
        let rust_tag = taxonomy_map.get_tag("rust").unwrap();
        assert_eq!(rust_tag.page_count, 2);

        // Check categories
        assert_eq!(taxonomy_map.categories().len(), 1);

        // Check series
        assert_eq!(taxonomy_map.series().len(), 1);
    }

    #[test]
    fn test_render_taxonomy_pages_no_templates() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a page with taxonomies
        let content = r#"
+++
title = "Post 1"
tags = ["rust", "web"]
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "post-1.md").unwrap();

        let route = RouteInfo::new(
            "/post-1/".to_string(),
            std::path::PathBuf::from("post-1.md"),
            std::path::PathBuf::from("post-1/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let processed = vec![ProcessedPage {
            route,
            page,
            html_content: "<p>Content</p>".to_string(),
        }];

        let taxonomy_map = build_taxonomy_map(&processed);

        // Use templates that don't have taxonomy templates
        let templates = TeraRenderer::from_dir(std::path::Path::new(
            "tests/fixtures/template_site/templates",
        ))
        .unwrap();

        let site_context = SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: None,
            author: None,
        };

        // Should return empty vec since no taxonomy templates exist
        let rendered =
            render_taxonomy_pages(&processed, &taxonomy_map, &templates, &site_context).unwrap();
        assert!(rendered.is_empty());
    }

    #[test]
    fn test_write_taxonomy_pages_dry_run() {
        let taxonomy_pages = vec![RenderedTaxonomy {
            path: "/tags/rust/".to_string(),
            output_file: PathBuf::from("tags/rust/index.html"),
            content: "<html>Test</html>".to_string(),
        }];

        // Dry run should not write anything
        let result = write_taxonomy_pages(&taxonomy_pages, Path::new("/nonexistent/path"), true);
        assert!(result.is_ok());
    }

    // ============================================
    // Co-located Asset Tests
    // ============================================

    #[test]
    fn test_copy_colocated_assets_basic() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("dist");

        // Create content directory with mixed files
        fs::create_dir_all(&content_dir).unwrap();
        fs::write(
            content_dir.join("post.md"),
            "+++\ntitle = \"Test\"\n+++\nContent",
        )
        .unwrap();
        fs::write(content_dir.join("photo.jpg"), "fake image data").unwrap();

        // Copy co-located assets
        let report = copy_colocated_assets(&content_dir, &output_dir, false).unwrap();

        // Should have copied 1 file (the .jpg)
        assert_eq!(report.files_processed, 1);

        // Verify the file was copied
        assert!(output_dir.join("photo.jpg").exists());

        // Verify the .md was NOT copied
        assert!(!output_dir.join("post.md").exists());
    }

    #[test]
    fn test_copy_colocated_assets_nested() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("dist");

        // Create nested directory structure
        fs::create_dir_all(content_dir.join("blog")).unwrap();
        fs::create_dir_all(content_dir.join("about")).unwrap();

        fs::write(
            content_dir.join("blog/post.md"),
            "+++\ntitle = \"Post\"\n+++\nContent",
        )
        .unwrap();
        fs::write(content_dir.join("blog/photo.jpg"), "image data").unwrap();
        fs::write(content_dir.join("about/headshot.png"), "png data").unwrap();

        // Copy co-located assets
        let report = copy_colocated_assets(&content_dir, &output_dir, false).unwrap();

        // Should have copied 2 files
        assert_eq!(report.files_processed, 2);

        // Verify nested paths are preserved
        assert!(output_dir.join("blog/photo.jpg").exists());
        assert!(output_dir.join("about/headshot.png").exists());

        // Verify .md was not copied
        assert!(!output_dir.join("blog/post.md").exists());
    }

    #[test]
    fn test_copy_colocated_assets_dry_run() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("dist");

        // Create content directory
        fs::create_dir_all(&content_dir).unwrap();
        fs::write(content_dir.join("image.png"), "png data").unwrap();

        // Dry run - should not write files
        let report = copy_colocated_assets(&content_dir, &output_dir, true).unwrap();

        // Should report 1 file processed
        assert_eq!(report.files_processed, 1);

        // But file should NOT exist in output
        assert!(!output_dir.join("image.png").exists());
    }

    #[test]
    fn test_copy_colocated_assets_empty_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("dist");

        // Create empty content directory
        fs::create_dir_all(&content_dir).unwrap();

        // Should succeed with 0 files
        let report = copy_colocated_assets(&content_dir, &output_dir, false).unwrap();
        assert_eq!(report.files_processed, 0);
    }

    #[test]
    fn test_copy_colocated_assets_nonexistent_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let content_dir = temp_dir.path().join("nonexistent");
        let output_dir = temp_dir.path().join("dist");

        // Should succeed with 0 files when directory doesn't exist
        let report = copy_colocated_assets(&content_dir, &output_dir, false).unwrap();
        assert_eq!(report.files_processed, 0);
    }

    #[test]
    fn test_copy_colocated_assets_various_extensions() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("dist");

        // Create content directory with various file types
        fs::create_dir_all(&content_dir).unwrap();
        fs::write(content_dir.join("image.jpg"), "jpg").unwrap();
        fs::write(content_dir.join("image.png"), "png").unwrap();
        fs::write(content_dir.join("image.gif"), "gif").unwrap();
        fs::write(content_dir.join("doc.pdf"), "pdf").unwrap();
        fs::write(content_dir.join("data.json"), "{}").unwrap();
        fs::write(content_dir.join("style.css"), "body {}").unwrap();
        fs::write(
            content_dir.join("page.md"),
            "+++\ntitle = \"Test\"\n+++\nContent",
        )
        .unwrap();

        // Copy co-located assets
        let report = copy_colocated_assets(&content_dir, &output_dir, false).unwrap();

        // Should have copied 6 files (all except .md)
        assert_eq!(report.files_processed, 6);

        // Verify all non-.md files were copied
        assert!(output_dir.join("image.jpg").exists());
        assert!(output_dir.join("image.png").exists());
        assert!(output_dir.join("image.gif").exists());
        assert!(output_dir.join("doc.pdf").exists());
        assert!(output_dir.join("data.json").exists());
        assert!(output_dir.join("style.css").exists());

        // Verify .md was not copied
        assert!(!output_dir.join("page.md").exists());
    }

    // ============================================
    // Internal Link Resolution Tests
    // ============================================

    #[test]
    fn test_resolve_internal_links_valid_link() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a registry with a route
        let mut registry = RouteRegistry::new();
        registry
            .register(
                RouteInfo::new(
                    "/about/".to_string(),
                    PathBuf::from("about.md"),
                    PathBuf::from("about/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();

        let content = "See my [about page](@/about.md) for more details.";
        let source_file = Path::new("blog/my-post.md");
        let result = resolve_internal_links(content, source_file, &registry).unwrap();

        assert_eq!(result, "See my [about page](/about/) for more details.");
    }

    #[test]
    fn test_resolve_internal_links_unknown_target() {
        // Create an empty registry
        let registry = RouteRegistry::new();

        let content = "See my [about page](@/about.md) for more details.";
        let source_file = Path::new("blog/my-post.md");
        let result = resolve_internal_links(content, source_file, &registry);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, BuildError::BrokenInternalLink { .. }));
        if let BuildError::BrokenInternalLink { file, target } = err {
            assert_eq!(file, "blog/my-post.md");
            assert_eq!(target, "@/about.md");
        }
    }

    #[test]
    fn test_resolve_internal_links_no_internal_links() {
        let registry = RouteRegistry::new();

        let content = "This is plain text with [a normal link](https://example.com).";
        let source_file = Path::new("test.md");
        let result = resolve_internal_links(content, source_file, &registry).unwrap();

        assert_eq!(
            result,
            "This is plain text with [a normal link](https://example.com)."
        );
    }

    #[test]
    fn test_resolve_internal_links_multiple_links() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a registry with multiple routes
        let mut registry = RouteRegistry::new();
        registry
            .register(
                RouteInfo::new(
                    "/about/".to_string(),
                    PathBuf::from("about.md"),
                    PathBuf::from("about/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();
        registry
            .register(
                RouteInfo::new(
                    "/blog/first-post/".to_string(),
                    PathBuf::from("blog/first-post.md"),
                    PathBuf::from("blog/first-post/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();

        let content = "See my [about page](@/about.md) and [first post](@/blog/first-post.md).";
        let source_file = Path::new("test.md");
        let result = resolve_internal_links(content, source_file, &registry).unwrap();

        assert_eq!(
            result,
            "See my [about page](/about/) and [first post](/blog/first-post/)."
        );
    }

    #[test]
    fn test_resolve_internal_links_nested_path() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a registry with a nested route
        let mut registry = RouteRegistry::new();
        registry
            .register(
                RouteInfo::new(
                    "/docs/guide/getting-started/".to_string(),
                    PathBuf::from("docs/guide/getting-started.md"),
                    PathBuf::from("docs/guide/getting-started/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();

        let content = "Read the [getting started guide](@/docs/guide/getting-started.md).";
        let source_file = Path::new("index.md");
        let result = resolve_internal_links(content, source_file, &registry).unwrap();

        assert_eq!(
            result,
            "Read the [getting started guide](/docs/guide/getting-started/)."
        );
    }

    #[test]
    fn test_resolve_internal_links_mixed_links() {
        use crate::routes::{RouteInfo, RouteKind};

        // Create a registry
        let mut registry = RouteRegistry::new();
        registry
            .register(
                RouteInfo::new(
                    "/about/".to_string(),
                    PathBuf::from("about.md"),
                    PathBuf::from("about/index.html"),
                    RouteKind::Page,
                )
                .unwrap(),
            )
            .unwrap();

        let content = "Check [external](https://example.com) and [internal](@/about.md) links.";
        let source_file = Path::new("test.md");
        let result = resolve_internal_links(content, source_file, &registry).unwrap();

        assert_eq!(
            result,
            "Check [external](https://example.com) and [internal](/about/) links."
        );
    }
}
