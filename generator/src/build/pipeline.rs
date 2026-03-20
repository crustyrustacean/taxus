//! Build pipeline stages.
//!
//! This module provides the individual stages of the build pipeline.

use crate::assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
use crate::config::SiteConfig;
use crate::content::Page;
use crate::error::{BuildError, GeneratorError, Result};
use crate::routes::{RouteDiscovery, RouteInfo, RouteRegistry};
use crate::templates::{PageContext, SiteContext, TemplateContext, TemplateRenderer, TeraRenderer};
use common::components::counter::{Counter, CounterProps};
use pulldown_cmark::{Parser, html::push_html};
use std::fs;
use std::path::Path;
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

        // Convert markdown to HTML
        let html_content = markdown_to_html(&page.raw_content);

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
    verbose: bool,
) -> Result<Vec<RenderedPage>> {
    let mut rendered = Vec::new();

    for processed_page in processed {
        let template_name = processed_page.page.template();

        // Build the template context
        let page_context = PageContext {
            title: processed_page.page.frontmatter.title.clone(),
            description: processed_page.page.frontmatter.description.clone(),
            path: processed_page.route.path.clone(),
            content: processed_page.html_content.clone(),
            raw_content: processed_page.page.raw_content.clone(),
            date: processed_page.page.frontmatter.date.map(|d| d.to_string()),
            draft: processed_page.page.is_draft(),
        };

        let context = TemplateContext::new(site_context.clone()).with_page(page_context);

        // Render the template
        let content = templates.render(template_name, &context).map_err(|e| {
            BuildError::PageRenderFailed {
                path: processed_page.route.path.clone(),
                source: e,
            }
        })?;

        if verbose {
            println!(
                "  Rendered: {} -> {}",
                processed_page.route.path, template_name
            );
        }

        rendered.push(RenderedPage {
            route: processed_page.route.clone(),
            content,
        });
    }

    Ok(rendered)
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

/// Write rendered pages to output files.
pub fn write_output(
    rendered: &[RenderedPage],
    output_dir: &Path,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    if dry_run {
        if verbose {
            println!("  Dry run - skipping file writes");
        }
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

        if verbose {
            println!("  Written: {}", output_path.display());
        }
    }

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

/// Convert markdown content to HTML.
fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut output = String::new();
    push_html(&mut output, parser);
    output
}

/// SSR a Yew island component and wrap it in the hydration mount div.
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
}
