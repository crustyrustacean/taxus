// taxus-generator/src/build/pipeline/taxonomy.rs

use crate::Page;
use crate::build::ProcessedPage;
use crate::content::{TaxonomyKind, TaxonomyMap};
use crate::error::{GeneratorError, Result};
use crate::templates::{
    PageContext, SiteContext, TaxonomyListContext, TaxonomyTermContext, TemplateContext,
    TemplateRenderer, TeraRenderer, compute_permalink,
};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

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

            let content = templates.render(&template_name, &context)?;

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
                            tagline: proc_page.page.frontmatter.tagline.clone(),
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

                let content = templates.render(&template_name, &context)?;

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
            fs::create_dir_all(parent).map_err(|e| GeneratorError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write the file
        fs::write(&output_path, &taxonomy_page.content).map_err(|e| GeneratorError::Io {
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

#[cfg(test)]
mod tests {

    use super::*;

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
}
