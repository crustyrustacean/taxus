// taxus-generator/src/build/pipeline.rs

//! Build pipeline stages.
//!
//! This module provides the individual stages of the build pipeline.

pub mod feeds;
pub mod internal_links;
pub mod not_found;
pub mod pages;
pub mod robots;
pub mod sitemap;

use crate::assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
use crate::build::pipeline::internal_links::resolve_internal_links;
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
use taxus_common::components::counter::{Counter, CounterProps};
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

/// Convert markdown content to HTML.
fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut output = String::new();
    push_html(&mut output, parser);
    output
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

/// SSR a Yew island component and wrap it in the hydration mount div.
///
/// Only compiled when the `islands` feature is enabled.
#[cfg(feature = "islands")]
pub fn render_island_counter(props: CounterProps) -> String {
    // Serialize props to JSON for the data attribute
    let props_json = serde_json::to_string(&props).unwrap_or_else(|_| "{}".to_string());

    // do a blocking call on this worker thread, move other tasks elsewhere
    let ssr_html = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            ServerRenderer::<Counter>::with_props(move || props)
                .render()
                .await
        })
    });

    // Emit the mount point wrapper around the SSR output
    format!(r#"<div data-island="Counter" data-props='{props_json}'>{ssr_html}</div>"#)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::pipeline::pages::render_pages;

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

    #[test]
    fn test_render_pages_with_pagination() {
        use crate::content::Frontmatter;
        use crate::routes::{RouteInfo, RouteKind};
        use std::path::PathBuf;

        let mut templates = TeraRenderer::new().unwrap();
        templates
            .register_template("page.html", r#"<h1>{{ page.title }}</h1>"#)
            .unwrap();
        templates
            .register_template(
                "section.html",
                r#"<h1>{{ section.title }}</h1>
{% if section.pagination %}
<div class="pagination">
  Page {{ section.pagination.current }} of {{ section.pagination.total }}
  ({{ section.pagination.total_items }} items)
  {% if section.pagination.prev %}<a href="{{ section.pagination.prev }}">Prev</a>{% endif %}
  {% if section.pagination.next %}<a href="{{ section.pagination.next }}">Next</a>{% endif %}
</div>
{% endif %}
<ul>
{% for p in section.pages %}
<li>{{ p.title }}</li>
{% endfor %}
</ul>"#,
            )
            .unwrap();

        let site_context = SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: None,
            author: None,
        };

        // Create a section with paginate_by = 2
        let section_route = RouteInfo::new(
            "/blog/".to_string(),
            PathBuf::from("blog/_index.md"),
            PathBuf::from("blog/index.html"),
            RouteKind::Section,
        )
        .unwrap();

        let section_page = ProcessedPage {
            route: section_route,
            page: Page {
                frontmatter: Frontmatter {
                    title: "Blog".to_string(),
                    template: Some("section.html".to_string()),
                    paginate_by: 2,
                    sort_by: crate::content::SortBy::Date,
                    ..Default::default()
                },
                path: "/blog/".to_string(),
                source: PathBuf::from("blog/_index.md"),
                raw_content: "Blog index".to_string(),
                content: None,
            },
            html_content: "<p>Blog index</p>".to_string(),
        };

        // Create 5 child pages
        let mut child_pages = Vec::new();
        for i in 1..=5 {
            let route = RouteInfo::new(
                format!("/blog/post-{}/", i),
                PathBuf::from(format!("blog/post-{}.md", i)),
                PathBuf::from(format!("blog/post-{}/index.html", i)),
                RouteKind::Page,
            )
            .unwrap();

            let date = chrono::NaiveDate::from_ymd_opt(2024, 1, i as u32);

            child_pages.push(ProcessedPage {
                route,
                page: Page {
                    frontmatter: Frontmatter {
                        title: format!("Post {}", i),
                        date,
                        ..Default::default()
                    },
                    path: format!("/blog/post-{}/", i),
                    source: PathBuf::from(format!("blog/post-{}.md", i)),
                    raw_content: format!("Content {}", i),
                    content: None,
                },
                html_content: format!("<p>Content {}</p>", i),
            });
        }

        let mut all_pages = vec![section_page];
        all_pages.extend(child_pages);

        let result = render_pages(&all_pages, &templates, &site_context, false).unwrap();

        // Should produce 3 paginated section pages + 5 child pages = 8 total
        // Page 1: posts 5, 4 (newest first, 2 per page)
        // Page 2: posts 3, 2
        // Page 3: post 1
        let section_pages: Vec<_> = result
            .iter()
            .filter(|r| r.route.path.starts_with("/blog/") && r.route.is_section())
            .collect();

        assert_eq!(section_pages.len(), 3, "Should have 3 paginated pages");

        // Check first page is at /blog/
        assert!(section_pages.iter().any(|r| r.route.path == "/blog/"));

        // Check second page is at /blog/page/2/
        assert!(
            section_pages
                .iter()
                .any(|r| r.route.path == "/blog/page/2/")
        );

        // Check third page is at /blog/page/3/
        assert!(
            section_pages
                .iter()
                .any(|r| r.route.path == "/blog/page/3/")
        );

        // Check pagination context is rendered
        let page1 = section_pages
            .iter()
            .find(|r| r.route.path == "/blog/")
            .unwrap();
        assert!(page1.content.contains("Page 1 of 3"));
        assert!(page1.content.contains("5 items"));
        assert!(!page1.content.contains("Prev")); // no prev on first page
        assert!(page1.content.contains("Next"));

        let page3 = section_pages
            .iter()
            .find(|r| r.route.path == "/blog/page/3/")
            .unwrap();
        assert!(page3.content.contains("Page 3 of 3"));
        assert!(page3.content.contains("Prev"));
        assert!(!page3.content.contains("Next")); // no next on last page
    }

    #[test]
    fn test_render_pages_no_pagination_when_not_configured() {
        use crate::content::Frontmatter;
        use crate::routes::{RouteInfo, RouteKind};
        use std::path::PathBuf;

        let mut templates = TeraRenderer::new().unwrap();
        templates
            .register_template("page.html", r#"<h1>{{ page.title }}</h1>"#)
            .unwrap();
        templates
            .register_template(
                "section.html",
                r#"<h1>{{ section.title }}</h1>
<ul>{% for p in section.pages %}<li>{{ p.title }}</li>{% endfor %}</ul>"#,
            )
            .unwrap();

        let site_context = SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: None,
            author: None,
        };

        // Section WITHOUT paginate_by
        let section_route = RouteInfo::new(
            "/blog/".to_string(),
            PathBuf::from("blog/_index.md"),
            PathBuf::from("blog/index.html"),
            RouteKind::Section,
        )
        .unwrap();

        let section_page = ProcessedPage {
            route: section_route,
            page: Page {
                frontmatter: Frontmatter {
                    title: "Blog".to_string(),
                    template: Some("section.html".to_string()),
                    paginate_by: 0, // no pagination
                    ..Default::default()
                },
                path: "/blog/".to_string(),
                source: PathBuf::from("blog/_index.md"),
                raw_content: "Blog".to_string(),
                content: None,
            },
            html_content: "<p>Blog</p>".to_string(),
        };

        let child_route = RouteInfo::new(
            "/blog/post-1/".to_string(),
            PathBuf::from("blog/post-1.md"),
            PathBuf::from("blog/post-1/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let child = ProcessedPage {
            route: child_route,
            page: Page {
                frontmatter: Frontmatter {
                    title: "Post 1".to_string(),
                    ..Default::default()
                },
                path: "/blog/post-1/".to_string(),
                source: PathBuf::from("blog/post-1.md"),
                raw_content: "Content".to_string(),
                content: None,
            },
            html_content: "<p>Content</p>".to_string(),
        };

        let all_pages = vec![section_page, child];

        let result = render_pages(&all_pages, &templates, &site_context, false).unwrap();

        // Should produce exactly 1 section page + 1 child page = 2 total
        let section_pages: Vec<_> = result.iter().filter(|r| r.route.is_section()).collect();

        assert_eq!(section_pages.len(), 1);
        assert_eq!(section_pages[0].route.path, "/blog/");
        assert!(section_pages[0].content.contains("Post 1"));
    }

    #[test]
    fn test_render_pages_with_extra_variables() {
        use crate::routes::{RouteInfo, RouteKind};

        let content = r#"
+++
title = "Styled Page"

[extra]
hero_image = "/images/hero.jpg"
css_class = "dark-theme"
featured = true
+++
Content here.
"#;
        let page = Page::from_str(content.trim_start(), "styled.md").unwrap();

        let route = RouteInfo::new(
            "/styled/".to_string(),
            std::path::PathBuf::from("styled.md"),
            std::path::PathBuf::from("styled/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let processed = ProcessedPage {
            route,
            page,
            html_content: "<p>Content here.</p>".to_string(),
        };

        let mut templates = TeraRenderer::new().unwrap();
        templates
            .register_template(
                "page.html",
                r#"<div class="{{ extra.css_class }}">
<img src="{{ extra.hero_image | safe }}" />
{% if extra.featured %}<span>Featured!</span>{% endif %}
<h1>{{ page.title }}</h1>
</div>"#,
            )
            .unwrap();

        let site_context = SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: None,
            author: None,
        };

        let rendered = render_pages(&[processed], &templates, &site_context, false).unwrap();

        assert_eq!(rendered.len(), 1);
        let html = &rendered[0].content;
        println!("Rendered HTML:\n{}", html);
        assert!(
            html.contains("dark-theme"),
            "Should contain css_class extra variable"
        );
        assert!(
            html.contains("/images/hero.jpg"),
            "Should contain hero_image extra variable"
        );
        assert!(
            html.contains("Featured!"),
            "Should contain featured extra variable"
        );
    }
}
