// taxus-generator/src/build/pipeline.rs

//! The build pipeline stages, one function per stage.
//!
//! Phase: parse ([`discover_tree`], [`discover_routes`]), then emit
//! ([`process_content`], [`process_images`], [`copy_colocated_assets`],
//! [`process_assets`], [`write_output`]). The stages that derive something
//! from the tree live in the submodules: [`pages`] (listings, pagination),
//! [`taxonomy`], [`feeds`] and [`sitemap`]. Stage numbers and what each
//! reads and produces are in the book's
//! [Architecture](https://crustyrustacean.github.io/taxus/architecture.html) chapter.
//!
//! Two intermediate types flow between stages: [`ProcessedPage`] (a
//! document after Markdown rendering) and [`RenderedPage`] (after its
//! template ran). Stages join tree nodes to processed pages by content
//! file, the one name a document keeps through every stage.

pub mod alias;
pub mod feeds;
pub mod internal_links;
pub mod markdown;
pub mod not_found;
pub mod pages;
pub mod robots;
pub mod search;
pub mod sitemap;
pub mod taxonomy;
pub mod wasm;

use crate::CodeHighlighter;
use crate::assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
use crate::build::pipeline::internal_links::resolve_internal_links;
use crate::build::ssr::block_on_ssr;
use crate::config::SiteConfig;
use crate::content::Page;
use crate::error::{GeneratorError, Result, RouteError};
use crate::images::{ImageProcessor, ImageRegistry, ProcessedImage};
use crate::routes::{RouteDiscovery, RouteInfo, RouteRegistry};
use crate::templates::TeraRenderer;
use std::fs;
use std::path::Path;
use taxus_domain::SiteTree;
use taxus_domain::UrlPath;
use taxus_domain::derivation::documents;
use tracing::{debug, debug_span, info, warn};

use taxus_common::components::counter::{Counter, CounterProps};
use taxus_common::components::search_box::SearchBoxProps;
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
    /// Table of contents extracted from the page's headings (#31)
    pub toc: Vec<markdown::TocEntry>,
    /// Processed hero image metadata (if page has hero_image)
    pub hero_image: Option<ProcessedImage>,
}

impl ProcessedPage {
    /// The URL path this page is served at.
    ///
    /// Derived from the Site Tree: the route's path is
    /// `UrlPath::from_node_path` of the page's membership path, so a
    /// frontmatter `slug` replaces the last segment and the page stays in
    /// its section (`content/blog/e.md` with `slug = "renamed-entry"` is
    /// `/blog/renamed-entry/`). Every consumer that emits a URL (feeds,
    /// sitemap, search index, templates, aliases) goes through this one
    /// accessor (#25).
    pub fn effective_url_path(&self) -> String {
        self.route.path.clone()
    }
}

/// Rendered page ready for writing.
#[derive(Debug, Clone)]
pub struct RenderedPage {
    /// Route information
    pub route: RouteInfo,
    /// Final HTML content
    pub content: String,
    /// Structured hero-image metadata (#17): the data behind the
    /// `<picture>` already embedded in `content`, kept for downstream
    /// consumers (reports, sitemap image entries, plugins) that need to
    /// inspect or transform it after rendering.
    pub hero_image: Option<ProcessedImage>,
}

/// Load configuration from a directory.
pub fn load_config(dir: &Path) -> std::result::Result<SiteConfig, GeneratorError> {
    SiteConfig::from_dir(dir)
}

/// Build the Site Tree from the content directory.
///
/// The tree is the source of truth for the build: the [`RouteRegistry`]
/// is derived from it with [`RouteRegistry::from_tree`], and stages that
/// need membership (section listings) query it. It is immutable once built.
pub fn discover_tree(config: &SiteConfig) -> Result<SiteTree> {
    RouteDiscovery::new(&config.build.content_dir).discover_tree()
}

/// The route registry the build runs on: [`discover_tree`] projected
/// through [`RouteRegistry::from_tree`].
///
/// For callers that only need routes (the `routes` CLI command, tests).
/// Every route's path is the served URL, slug overrides included.
pub fn discover_routes(config: &SiteConfig) -> Result<RouteRegistry> {
    Ok(RouteRegistry::from_tree(&discover_tree(config)?))
}

/// Load templates from the templates directory.
pub fn load_templates(config: &SiteConfig) -> Result<TeraRenderer> {
    Ok(TeraRenderer::from_dir(&config.build.templates_dir)?)
}

/// Process content files into rendered HTML.
///
/// The tree is the whole model (#55, Phase 3 of the seam-closure plan):
/// discovery already parsed every file into tree nodes — frontmatter and
/// raw body — so this stage re-reads nothing from disk. It walks the
/// documents in canonical tree order, joins each to its route by node
/// path, and counts skips as they happen (drafts today, anything else
/// tomorrow) rather than inferring them by subtraction.
///
/// Returns the processed pages and the number of documents skipped.
pub fn process_content(
    tree: &SiteTree,
    registry: &RouteRegistry,
    config: &SiteConfig,
    include_drafts: bool,
    mut highlighter: Option<&mut CodeHighlighter>,
) -> Result<(Vec<ProcessedPage>, usize)> {
    let mut pages = Vec::new();
    let mut skipped = 0usize;

    for node in documents(tree) {
        // The skip paths live here, counted where they happen.
        if node.is_draft() && !include_drafts {
            skipped += 1;
            continue;
        }

        // The route for this document, joined by node path — the same
        // identity `RouteRegistry::from_tree` registered it under.
        let route = registry
            .get(&UrlPath::from_node_path(node.path()).to_string())
            .cloned()
            .ok_or_else(|| {
                GeneratorError::Route(Box::new(RouteError::InvalidPath(format!(
                    "tree node {} has no route",
                    node.path()
                ))))
            })?;

        let page = Page {
            frontmatter: node.meta().clone(),
            raw_content: node.body().to_string(),
        };

        // Resolve internal links in the content
        let resolved_content =
            resolve_internal_links(&page.raw_content, &route.content_file, registry)?;

        // Convert markdown to HTML (collecting heading TOC at the same time)
        let md_options = markdown::MarkdownOptions {
            insert_anchor_links: config.markdown.insert_anchor_links,
        };
        let (html_content, toc) = markdown::markdown_to_html_with_toc(
            &resolved_content,
            highlighter.as_deref_mut(),
            &md_options,
        );

        pages.push(ProcessedPage {
            route,
            page,
            html_content,
            toc,
            hero_image: None,
        });
    }

    Ok((pages, skipped))
}

/// Process assets (SCSS and static files).
///
/// In dry-run mode, SCSS is still compiled (so errors surface) but no
/// output files are written and nothing is copied.
pub fn process_assets(
    config: &SiteConfig,
    output_dir: &Path,
    dry_run: bool,
) -> Result<AssetReport> {
    let mut report = AssetReport::new();

    // Process SCSS files
    if config.build.styles_dir.exists() {
        let scss_processor = ScssProcessor::new();
        let css_output = output_dir.join("css");
        let scss_report = scss_processor.process(&config.build.styles_dir, &css_output, dry_run)?;
        report.merge(scss_report);
    }

    // Copy static files
    if config.build.static_dir.exists() {
        let static_copier = StaticCopier::new();
        let static_output = output_dir.join("static");
        let static_report =
            static_copier.process(&config.build.static_dir, &static_output, dry_run)?;
        report.merge(static_report);
    }

    Ok(report)
}

/// Process hero images for pages that have `hero_image` in frontmatter.
///
/// Walks all `ProcessedPage`s, resolves co-located image paths, and generates
/// responsive variants. Stores results in the `ImageRegistry` and attaches
/// `ProcessedImage` metadata to each `ProcessedPage`.
pub fn process_images(
    processed: &mut [ProcessedPage],
    config: &SiteConfig,
    dry_run: bool,
) -> Result<ImageRegistry> {
    let mut registry = ImageRegistry::new();
    let processor = ImageProcessor::new(config.images.clone(), config.build.output_dir.clone());

    // Once per build, not per image (#34).
    if processor.quality_ignored_for_webp()
        && processed
            .iter()
            .any(|p| p.page.frontmatter.hero_image.is_some())
    {
        warn!(
            quality = config.images.quality,
            "images.quality is ignored: WebP variants are lossless because taxus was built \
             without the `webp-lossy` feature"
        );
    }

    for page in processed.iter_mut() {
        if let Some(ref hero_image_path) = page.page.frontmatter.hero_image {
            let content_dir = if let Some(parent) = page.route.content_file.parent() {
                config.build.content_dir.join(parent)
            } else {
                config.build.content_dir.clone()
            };
            let source_path = content_dir.join(hero_image_path);

            let alt = page
                .page
                .frontmatter
                .hero_alt
                .as_deref()
                .or(Some(&page.page.frontmatter.title))
                .unwrap_or("")
                .to_string();

            let result = if dry_run {
                processor.process_dry(&source_path, &alt)?
            } else {
                processor.process(&source_path, &alt)?
            };

            registry.insert(source_path.clone(), result.clone());
            page.hero_image = Some(result);
        }
    }

    Ok(registry)
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
            fs::create_dir_all(parent).map_err(|e| GeneratorError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Copy the file
        fs::copy(path, &dest_path).map_err(|e| GeneratorError::Io {
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
pub fn write_output(rendered: &[RenderedPage], output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping file writes");
        return Ok(());
    }

    for rendered_page in rendered {
        let output_path = output_dir.join(&rendered_page.route.output_file);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| GeneratorError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write the file
        fs::write(&output_path, &rendered_page.content).map_err(|e| GeneratorError::Io {
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

/// Clean the output directory.
pub fn clean_output(output_dir: &Path) -> Result<()> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir).map_err(|e| GeneratorError::Io {
            path: output_dir.to_path_buf(),
            source: e,
        })?;
    }
    Ok(())
}

/// SSR a Yew island component and wrap it in the hydration mount div.
pub fn render_island_counter(props: CounterProps) -> String {
    // Serialize props to JSON for the data attribute
    let props_json = serde_json::to_string(&props).unwrap_or_else(|_| "{}".to_string());

    // Drive the SSR future on the dedicated island thread so this works from
    // any calling context, with or without a tokio runtime (#37).
    let ssr_html = block_on_ssr(ServerRenderer::<Counter>::with_props(move || props).render());

    // Emit the mount point wrapper around the SSR output
    island_mount("Counter", &props_json, &ssr_html)
}

pub fn render_search_box(props: SearchBoxProps) -> String {
    use taxus_common::components::search_box::SearchBox;

    let props_json = serde_json::to_string(&props).unwrap_or_else(|_| "{}".to_string());

    let ssr_html = block_on_ssr(ServerRenderer::<SearchBox>::with_props(move || props).render());

    island_mount("SearchBox", &props_json, &ssr_html)
}

/// The hydration mount point for an island (#39).
///
/// `data-props` carries the serialized props inside a single-quoted
/// attribute, so the JSON must be HTML-escaped first: a `'` in any prop
/// value would otherwise terminate the attribute early and leave the
/// client parsing a truncated blob (falling back to default props).
/// The browser's `dataset` accessor un-escapes entities when the client
/// reads the attribute back, so `taxus-client` needs no change.
fn island_mount(name: &str, props_json: &str, ssr_html: &str) -> String {
    let escaped = escape_attribute(props_json);
    format!(r#"<div data-island="{name}" data-props='{escaped}'>{ssr_html}</div>"#)
}

/// Escape a value for embedding in a single-quoted HTML attribute.
fn escape_attribute(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '\'' => out.push_str("&#39;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Test helpers shared by the stage modules.
#[cfg(test)]
pub(crate) mod test_support {
    use super::ProcessedPage;
    use taxus_domain::{NodePath, SiteTree, SiteTreeBuilder};

    /// Build the tree the way discovery would for these processed pages.
    pub(crate) fn tree_of(processed: &[ProcessedPage]) -> SiteTree {
        let mut builder = SiteTreeBuilder::new();
        for p in processed {
            let path = NodePath::parse(&p.route.path).unwrap();
            let meta = p.page.frontmatter.clone();
            let body = p.page.raw_content.clone();
            let file = p.route.content_file.clone();
            if p.route.is_section() {
                if path.is_root() {
                    builder = builder.root(Some(file), meta, Some(body));
                } else {
                    builder
                        .add_section(&path, Some(file), meta, Some(body))
                        .unwrap();
                }
            } else {
                builder.add_page(&path, file, meta, body).unwrap();
            }
        }
        builder.build().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
