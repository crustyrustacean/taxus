// taxus-generator/src/build/pipeline/pages.rs

use crate::build::{ProcessedPage, RenderedPage};
use crate::content::SortBy;
use crate::error::{BuildError, Result};
use crate::routes::RouteInfo;
use crate::templates::{
    PageContext, PaginationContext, SectionContext, SiteContext, TemplateContext, TemplateRenderer,
    TeraRenderer, compute_permalink,
};
use std::path::PathBuf;
use tracing::{debug, debug_span, info};

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
        // Use section.html as default for sections, page.html for pages
        let template_name = if processed_page.route.is_section() {
            processed_page
                .page
                .frontmatter
                .template
                .as_deref()
                .unwrap_or("section.html")
        } else {
            processed_page.page.template()
        };

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

        // Build context, adding section context for section routes
        // Note: Root index ("/") is a special case - it doesn't list all site pages
        let is_root_index = processed_page.route.path == "/";
        let context = if processed_page.route.is_section() {
            // Find all child pages for this section (skip for root index)
            let mut child_pages: Vec<PageContext> = if is_root_index {
                Vec::new()
            } else {
                processed
                    .iter()
                    .filter(|p| {
                        p.route.is_page()
                            && p.route.path.starts_with(&processed_page.route.path)
                            && p.route.path != processed_page.route.path
                    })
                    .map(|p| {
                        let child_url_path = if p.page.frontmatter.slug.is_some() {
                            p.page.url_path()
                        } else {
                            p.route.path.clone()
                        };
                        let child_permalink =
                            compute_permalink(&site_context.base_url, &child_url_path);
                        PageContext {
                            title: p.page.frontmatter.title.clone(),
                            description: p.page.frontmatter.description.clone(),
                            path: child_url_path,
                            permalink: child_permalink,
                            content: p.html_content.clone(),
                            raw_content: p.page.raw_content.clone(),
                            date: p.page.frontmatter.date.map(|d| d.to_string()),
                            draft: p.page.is_draft(),
                            summary: p.page.summary(),
                            word_count: p.page.word_count(),
                            reading_time: p.page.reading_time(),
                            tags: p.page.tags().to_vec(),
                            categories: p.page.categories().to_vec(),
                            series: p.page.series().map(|s| s.to_string()),
                        }
                    })
                    .collect()
            };

            // Sort according to section's sort_by field
            sort_page_contexts(&mut child_pages, processed_page.page.frontmatter.sort_by);

            // Check if this section uses pagination
            let paginate_by = processed_page.page.frontmatter.paginate_by;

            if paginate_by > 0 && !child_pages.is_empty() {
                // ── Paginated rendering ──
                let total_items = child_pages.len();
                let total_pages = child_pages.len().div_ceil(paginate_by);

                let paginate_template = processed_page
                    .page
                    .frontmatter
                    .paginate_template
                    .as_deref()
                    .or(processed_page.page.frontmatter.template.as_deref())
                    .unwrap_or("section.html");

                for page_num in 1..=total_pages {
                    let start = (page_num - 1) * paginate_by;
                    let end = std::cmp::min(start + paginate_by, total_items);
                    let slice = child_pages[start..end].to_vec();

                    // Build pagination URLs
                    let base_path = url_path.trim_end_matches('/');
                    let page_url = |n: usize| -> String {
                        if n == 1 {
                            format!("{}/", base_path)
                        } else {
                            format!("{}/page/{}/", base_path, n)
                        }
                    };

                    let pagination_context = PaginationContext {
                        current: page_num,
                        total: total_pages,
                        per_page: paginate_by,
                        total_items,
                        prev: if page_num > 1 {
                            Some(page_url(page_num - 1))
                        } else {
                            None
                        },
                        next: if page_num < total_pages {
                            Some(page_url(page_num + 1))
                        } else {
                            None
                        },
                        first: page_url(1),
                        last: page_url(total_pages),
                    };

                    let section_context = SectionContext {
                        title: processed_page.page.frontmatter.title.clone(),
                        description: processed_page.page.frontmatter.description.clone(),
                        path: page_url(page_num),
                        content: Some(processed_page.html_content.clone()),
                        pages: slice,
                        pagination: Some(pagination_context),
                    };

                    // Clone page_context for each paginated page
                    let paginated_page_context = PageContext {
                        title: processed_page.page.frontmatter.title.clone(),
                        description: processed_page.page.frontmatter.description.clone(),
                        path: page_url(page_num),
                        permalink: compute_permalink(&site_context.base_url, &page_url(page_num)),
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

                    let context = TemplateContext::new(site_context.clone())
                        .with_page(paginated_page_context)
                        .with_section(section_context)
                        .with_extra(processed_page.page.frontmatter.extra_as_json());

                    let content = templates.render(paginate_template, &context).map_err(|e| {
                        BuildError::PageRenderFailed {
                            path: page_url(page_num),
                            source: e,
                        }
                    })?;

                    // Determine output file path
                    let output_file = if page_num == 1 {
                        processed_page.route.output_file.clone()
                    } else {
                        let trimmed = url_path.trim_start_matches('/').trim_end_matches('/');
                        if trimmed.is_empty() {
                            PathBuf::from(format!("page/{}/index.html", page_num))
                        } else {
                            PathBuf::from(trimmed).join(format!("page/{}/index.html", page_num))
                        }
                    };

                    let route = RouteInfo::new(
                        page_url(page_num),
                        processed_page.route.content_file.clone(),
                        output_file,
                        processed_page.route.kind,
                    )
                    .map_err(BuildError::from)?;

                    rendered.push(RenderedPage { route, content });
                }

                // Skip the normal rendering below — we already pushed all paginated pages
                continue;
            }

            // ── Non-paginated section (original behavior) ──
            let section_context = SectionContext {
                title: processed_page.page.frontmatter.title.clone(),
                description: processed_page.page.frontmatter.description.clone(),
                path: url_path.clone(),
                content: Some(processed_page.html_content.clone()),
                pages: child_pages,
                pagination: None,
            };

            TemplateContext::new(site_context.clone())
                .with_page(page_context)
                .with_section(section_context)
                .with_extra(processed_page.page.frontmatter.extra_as_json())
        } else {
            TemplateContext::new(site_context.clone())
                .with_page(page_context)
                .with_extra(processed_page.page.frontmatter.extra_as_json())
        };

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

/// Sort page contexts according to a sort order.
fn sort_page_contexts(pages: &mut [PageContext], sort_by: SortBy) {
    match sort_by {
        SortBy::Date => {
            pages.sort_by(|a, b| match (&b.date, &a.date) {
                (Some(date_b), Some(date_a)) => date_b.cmp(date_a),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
        SortBy::Title => {
            pages.sort_by(|a, b| a.title.cmp(&b.title));
        }
        SortBy::Weight => {
            // Weight isn't in PageContext, so fall back to title sort
            // This is a limitation - weight-based sorting requires the raw page data
            pages.sort_by(|a, b| a.title.cmp(&b.title));
        }
        SortBy::None => {} // preserve existing order
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::content::Page;

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
    fn test_render_pages_section_with_child_pages() {
        use crate::content::Frontmatter;
        use crate::routes::{RouteInfo, RouteKind};
        use crate::templates::TeraRenderer;
        use std::path::PathBuf;

        // Create a simple template that outputs section pages
        let mut templates = TeraRenderer::new().unwrap();
        templates
            .register_template("page.html", r#"<h1>{{ page.title }}</h1>"#)
            .unwrap();
        templates
            .register_template(
                "section.html",
                r#"<h1>{{ page.title }}</h1>
<ul>
{% for p in section.pages %}
<li>{{ p.title }} - {{ p.date | default(value="no date") }}</li>
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

        // Create a section route (blog index)
        let section_route = RouteInfo::new(
            "/blog/".to_string(),
            PathBuf::from("blog/_index.md"),
            PathBuf::from("blog/index.html"),
            RouteKind::Section,
        )
        .unwrap();

        // Create child page routes
        let page1_route = RouteInfo::new(
            "/blog/first-post/".to_string(),
            PathBuf::from("blog/first-post.md"),
            PathBuf::from("blog/first-post/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let page2_route = RouteInfo::new(
            "/blog/second-post/".to_string(),
            PathBuf::from("blog/second-post.md"),
            PathBuf::from("blog/second-post/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        // Create processed pages
        let section_page = ProcessedPage {
            route: section_route,
            page: Page {
                frontmatter: Frontmatter {
                    title: "Blog".to_string(),
                    description: None,
                    date: None,
                    draft: false,
                    slug: None,
                    template: Some("section.html".to_string()),
                    summary: None,
                    aliases: vec![],
                    tags: vec![],
                    categories: vec![],
                    series: None,
                    extra: None,
                    sort_by: Default::default(),
                    paginate_by: 0,
                    paginate_template: None,
                    weight: 0,
                    updated: None,
                },
                path: "/blog/".to_string(),
                source: PathBuf::from("blog/_index.md"),
                raw_content: "Blog index".to_string(),
                content: None,
            },
            html_content: "<p>Blog index</p>".to_string(),
        };

        let page1 = ProcessedPage {
            route: page1_route,
            page: Page {
                frontmatter: Frontmatter {
                    title: "First Post".to_string(),
                    description: None,
                    date: chrono::NaiveDate::from_ymd_opt(2024, 1, 15),
                    draft: false,
                    slug: None,
                    template: None,
                    summary: None,
                    aliases: vec![],
                    tags: vec![],
                    categories: vec![],
                    series: None,
                    extra: None,
                    sort_by: Default::default(),
                    paginate_by: 0,
                    paginate_template: None,
                    weight: 0,
                    updated: None,
                },
                path: "/blog/first-post/".to_string(),
                source: PathBuf::from("blog/first-post.md"),
                raw_content: "First post content".to_string(),
                content: None,
            },
            html_content: "<p>First post content</p>".to_string(),
        };

        let page2 = ProcessedPage {
            route: page2_route,
            page: Page {
                frontmatter: Frontmatter {
                    title: "Second Post".to_string(),
                    description: None,
                    date: chrono::NaiveDate::from_ymd_opt(2024, 2, 20),
                    draft: false,
                    slug: None,
                    template: None,
                    summary: None,
                    aliases: vec![],
                    tags: vec![],
                    categories: vec![],
                    series: None,
                    extra: None,
                    sort_by: Default::default(),
                    paginate_by: 0,
                    paginate_template: None,
                    weight: 0,
                    updated: None,
                },
                path: "/blog/second-post/".to_string(),
                source: PathBuf::from("blog/second-post.md"),
                raw_content: "Second post content".to_string(),
                content: None,
            },
            html_content: "<p>Second post content</p>".to_string(),
        };

        let processed = vec![section_page, page1, page2];

        // Render pages
        let result = render_pages(&processed, &templates, &site_context, false).unwrap();

        // Find the section page
        let section_rendered = result
            .iter()
            .find(|r| r.route.path == "/blog/")
            .expect("Section page should be rendered");

        // Verify the section context was populated
        assert!(section_rendered.content.contains("Second Post"));
        assert!(section_rendered.content.contains("First Post"));

        // Verify sorting: newest first (Second Post should appear before First Post)
        let second_pos = section_rendered.content.find("Second Post").unwrap();
        let first_pos = section_rendered.content.find("First Post").unwrap();
        assert!(
            second_pos < first_pos,
            "Second Post (newer) should appear before First Post (older)"
        );
    }

    #[test]
    fn test_render_pages_section_without_child_pages() {
        use crate::content::Frontmatter;
        use crate::routes::{RouteInfo, RouteKind};
        use crate::templates::TeraRenderer;
        use std::path::PathBuf;

        // Create a simple template
        let mut templates = TeraRenderer::new().unwrap();
        templates
            .register_template(
                "section.html",
                r#"<h1>{{ page.title }}</h1>
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

        // Create a section route with no children
        let section_route = RouteInfo::new(
            "/empty/".to_string(),
            PathBuf::from("empty/_index.md"),
            PathBuf::from("empty/index.html"),
            RouteKind::Section,
        )
        .unwrap();

        let section_page = ProcessedPage {
            route: section_route,
            page: Page {
                frontmatter: Frontmatter {
                    title: "Empty Section".to_string(),
                    description: None,
                    date: None,
                    draft: false,
                    slug: None,
                    template: Some("section.html".to_string()),
                    summary: None,
                    aliases: vec![],
                    tags: vec![],
                    categories: vec![],
                    series: None,
                    extra: None,
                    sort_by: Default::default(),
                    paginate_by: 0,
                    paginate_template: None,
                    weight: 0,
                    updated: None,
                },
                path: "/empty/".to_string(),
                source: PathBuf::from("empty/_index.md"),
                raw_content: "Empty section".to_string(),
                content: None,
            },
            html_content: "<p>Empty section</p>".to_string(),
        };

        let processed = vec![section_page];

        // Render pages
        let result = render_pages(&processed, &templates, &site_context, false).unwrap();

        // Find the section page
        let section_rendered = result
            .iter()
            .find(|r| r.route.path == "/empty/")
            .expect("Section page should be rendered");

        // Verify the section context has empty pages list
        assert!(section_rendered.content.contains("<ul>"));
        assert!(section_rendered.content.contains("</ul>"));
        // Should not contain any list items
        assert!(!section_rendered.content.contains("<li>"));
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
