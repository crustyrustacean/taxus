// taxus-generator/src/build/pipeline/pages.rs

use crate::build::{ProcessedPage, RenderedPage};
use crate::content::SortBy;
use crate::error::Result;
use crate::routes::RouteInfo;
use crate::templates::{
    HeroContext, PageContext, PaginationContext, SectionContext, SiteContext, TemplateContext,
    TemplateRenderer, TeraRenderer, compute_permalink,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use taxus_domain::{NodePath, PageNode, SiteTree, derivation};
use tracing::{debug, debug_span, info, warn};

/// Build a `PageContext` from a `ProcessedPage`.
///
/// Handles the slug-vs-route-path logic: if a custom slug is defined in frontmatter,
/// uses the slug-derived URL path; otherwise uses the discovered route path.
/// Computes the permalink from the base URL and resolved path.
fn page_context_from(processed: &ProcessedPage, base_url: &str) -> PageContext {
    let url_path = processed.effective_url_path();
    let permalink = compute_permalink(base_url, &url_path);
    let hero = processed.hero_image.as_ref().map(|img| HeroContext {
        src: img.fallback_src(),
        srcset: img.srcset(),
        width: img.meta.original_width,
        height: img.meta.original_height,
        alt: img.meta.alt.clone(),
        mime_type: img.mime_type(),
    });
    PageContext {
        title: processed.page.frontmatter.title.clone(),
        description: processed.page.frontmatter.description.clone(),
        tagline: processed.page.frontmatter.tagline.clone(),
        path: url_path,
        permalink,
        content: processed.html_content.clone(),
        toc: processed.toc.clone(),
        raw_content: processed.page.raw_content.clone(),
        date: processed.page.frontmatter.date.map(|d| d.to_string()),
        draft: processed.page.is_draft(),
        summary: processed.page.summary(),
        word_count: processed.page.word_count(),
        reading_time: processed.page.reading_time(),
        tags: processed.page.tags().to_vec(),
        categories: processed.page.categories().to_vec(),
        series: processed.page.series().map(|s| s.to_string()),
        weight: processed.page.frontmatter.weight,
        hero,
    }
}

/// Collect the pages a section lists, in listing order.
///
/// Membership comes from the tree and is **direct children only**: the
/// section node's `pages`, plus the direct pages of every section named in
/// the section's `pages_from` frontmatter ([`derivation::aggregate`]).
/// Deeper descendants are not listed, so the root lists the site only if
/// its `_index.md` declares `pages_from` (#70). Pages the build skipped
/// (drafts) are dropped. Ordering is [`sort_section_pages`].
fn collect_child_pages(
    section: &ProcessedPage,
    tree: &SiteTree,
    processed_by_file: &HashMap<&Path, &ProcessedPage>,
    base_url: &str,
) -> Vec<PageContext> {
    let node = NodePath::parse(&section.route.path)
        .ok()
        .and_then(|path| tree.get_section(&path));
    let Some(node) = node else {
        debug!(path = %section.route.path, "Section route has no tree node; listing nothing");
        return Vec::new();
    };

    let mut donors = Vec::new();
    for raw in &section.page.frontmatter.pages_from {
        match NodePath::parse(raw) {
            Ok(path) if tree.get_section(&path).is_some() => donors.push(path),
            Ok(_) => warn!(
                section = %section.route.path,
                pages_from = %raw,
                "pages_from names a section that does not exist; ignoring it"
            ),
            Err(e) => warn!(
                section = %section.route.path,
                pages_from = %raw,
                error = %e,
                "pages_from entry is not a valid section path; ignoring it"
            ),
        }
    }

    let mut pages = derivation::aggregate(node, tree, &donors);
    sort_section_pages(&mut pages, section.page.frontmatter.sort_by);
    pages
        .iter()
        .filter_map(|node| processed_by_file.get(node.content_file.as_path()))
        .map(|p| page_context_from(p, base_url))
        .collect()
}

/// Order a section's pages for listing.
///
/// `Weight` and `None` delegate to [`taxus_domain::tree::sort_pages`]:
/// lowest weight first (#5), ties and `None` in the tree's slug order.
///
/// `Date` and `Title` deliberately keep trunk's comparators rather than
/// the domain's, so generated output is unchanged by the tree port:
/// - `Date`: newest first, but *undated pages first* (the domain puts
///   them last);
/// - `Title`: byte order (the domain compares case-insensitively).
///
/// Switching those two to the domain ordering is a visible behaviour
/// change and belongs to its own PR with a golden re-baseline.
fn sort_section_pages(pages: &mut [&PageNode], sort_by: SortBy) {
    match sort_by {
        SortBy::Date => pages.sort_by(|a, b| match (&b.meta.date, &a.meta.date) {
            (Some(date_b), Some(date_a)) => date_b.cmp(date_a),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }),
        SortBy::Title => pages.sort_by(|a, b| a.meta.title.cmp(&b.meta.title)),
        SortBy::Weight | SortBy::None => taxus_domain::tree::sort_pages(pages, sort_by),
    }
}

/// Render a paginated section.
///
/// Handles the entire pagination loop: slices child pages per pagination page,
/// builds `PaginationContext`, renders each page with the template, and collects
/// results. Uses `page_context_from` internally, overriding `path` and `permalink`
/// for each pagination page. First page outputs to the section's normal path;
/// subsequent pages go to `/page/N/` subdirectories.
fn render_paginated_section(
    processed_page: &ProcessedPage,
    child_pages: Vec<PageContext>,
    templates: &TeraRenderer,
    site_context: &SiteContext,
    url_path: &str,
) -> Result<Vec<RenderedPage>> {
    let mut rendered = Vec::new();
    let paginate_by = processed_page.page.frontmatter.paginate_by;
    let total_items = child_pages.len();
    let total_pages = total_items.div_ceil(paginate_by);

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
            permalink: compute_permalink(&site_context.base_url, &page_url(page_num)),
            content: Some(processed_page.html_content.clone()),
            toc: processed_page.toc.clone(),
            pages: slice,
            pagination: Some(pagination_context),
        };

        let mut paginated_page_context = page_context_from(processed_page, &site_context.base_url);
        paginated_page_context.path = page_url(page_num);
        paginated_page_context.permalink =
            compute_permalink(&site_context.base_url, &page_url(page_num));

        let context = TemplateContext::new(site_context.clone())
            .with_page(paginated_page_context)
            .with_section(section_context)
            .with_extra(processed_page.page.frontmatter.extra_as_json());

        let content = templates.render(paginate_template, &context)?;

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
        )?;

        rendered.push(RenderedPage { route, content });
    }

    Ok(rendered)
}

/// Render pages using templates.
///
/// Iterates through processed pages, builds template contexts, and renders each
/// to HTML. For sections, the listed pages come from `tree` (see
/// [`collect_child_pages`]); paginated sections are dispatched to
/// `render_paginated_section`. Regular pages and non-paginated sections are
/// rendered directly.
pub fn render_pages(
    processed: &[ProcessedPage],
    tree: &SiteTree,
    templates: &TeraRenderer,
    site_context: &SiteContext,
    _verbose: bool,
) -> Result<Vec<RenderedPage>> {
    let span = debug_span!("render_pages", pages = processed.len());
    let _enter = span.enter();

    // Tree nodes are joined back to their processed pages by content file:
    // storage identity, shared by both, and absent for skipped drafts.
    let processed_by_file: HashMap<&Path, &ProcessedPage> = processed
        .iter()
        .filter(|p| p.route.is_page())
        .map(|p| (p.route.content_file.as_path(), p))
        .collect();

    let mut rendered = Vec::new();

    for processed_page in processed {
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

        let url_path = processed_page.effective_url_path();

        debug!(path = %url_path, template = %template_name, "Rendering page");

        let page_context = page_context_from(processed_page, &site_context.base_url);

        let context = if processed_page.route.is_section() {
            let child_pages = collect_child_pages(
                processed_page,
                tree,
                &processed_by_file,
                &site_context.base_url,
            );

            let paginate_by = processed_page.page.frontmatter.paginate_by;
            if paginate_by > 0 && !child_pages.is_empty() {
                rendered.extend(render_paginated_section(
                    processed_page,
                    child_pages,
                    templates,
                    site_context,
                    &url_path,
                )?);
                continue;
            }

            let section_context = SectionContext {
                title: processed_page.page.frontmatter.title.clone(),
                description: processed_page.page.frontmatter.description.clone(),
                path: url_path.clone(),
                permalink: compute_permalink(&site_context.base_url, &url_path),
                content: Some(processed_page.html_content.clone()),
                toc: processed_page.toc.clone(),
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

        let content = templates.render(template_name, &context)?;

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
        )?;

        rendered.push(RenderedPage { route, content });
    }

    info!("Rendered {} pages", rendered.len());
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Frontmatter, Page};
    use crate::routes::{RouteInfo, RouteKind};
    use std::path::PathBuf;

    use crate::build::pipeline::test_support::tree_of;

    fn test_site_context() -> SiteContext {
        SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: None,
            author: None,
        }
    }

    // ── Slug tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_render_pages_with_custom_slug() {
        let content = r#"
+++
title = "Test Post"
slug = "custom-url"
+++
This is the content.
"#;
        let page = Page::from_str(content.trim_start(), "original-filename.md").unwrap();

        let route = RouteInfo::new(
            "/original-filename/".to_string(),
            PathBuf::from("original-filename.md"),
            PathBuf::from("original-filename/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let processed = ProcessedPage {
            route,
            page,
            html_content: "<p>This is the content.</p>".to_string(),
            toc: Vec::new(),
            hero_image: None,
        };

        let templates = TeraRenderer::from_dir(std::path::Path::new(
            "tests/fixtures/template_site/templates",
        ))
        .unwrap();

        let processed = [processed];
        let rendered = render_pages(
            &processed,
            &tree_of(&processed),
            &templates,
            &test_site_context(),
            false,
        )
        .unwrap();

        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].route.path, "/custom-url/");
        assert_eq!(
            rendered[0].route.output_file,
            PathBuf::from("custom-url/index.html")
        );
    }

    #[test]
    fn test_render_pages_without_custom_slug() {
        let content = r#"
+++
title = "Test Post"
+++
This is the content.
"#;
        let page = Page::from_str(content.trim_start(), "my-post.md").unwrap();

        let route = RouteInfo::new(
            "/my-post/".to_string(),
            PathBuf::from("my-post.md"),
            PathBuf::from("my-post/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let processed = ProcessedPage {
            route,
            page,
            html_content: "<p>This is the content.</p>".to_string(),
            toc: Vec::new(),
            hero_image: None,
        };

        let templates = TeraRenderer::from_dir(std::path::Path::new(
            "tests/fixtures/template_site/templates",
        ))
        .unwrap();

        let processed = [processed];
        let rendered = render_pages(
            &processed,
            &tree_of(&processed),
            &templates,
            &test_site_context(),
            false,
        )
        .unwrap();

        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].route.path, "/my-post/");
        assert_eq!(
            rendered[0].route.output_file,
            PathBuf::from("my-post/index.html")
        );
    }

    // ── Section rendering tests ────────────────────────────────────────────────

    #[test]
    fn test_render_pages_section_with_child_pages() {
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

        let section_page = {
            let route = RouteInfo::new(
                "/blog/".to_string(),
                PathBuf::from("blog/_index.md"),
                PathBuf::from("blog/index.html"),
                RouteKind::Section,
            )
            .unwrap();
            ProcessedPage {
                toc: Vec::new(),
                route,
                page: Page {
                    frontmatter: Frontmatter {
                        title: "Blog".to_string(),
                        template: Some("section.html".to_string()),
                        ..Default::default()
                    },
                    path: "/blog/".to_string(),
                    source: PathBuf::from("blog/_index.md"),
                    raw_content: "Blog index".to_string(),
                    content: None,
                },
                html_content: "<p>Blog index</p>".to_string(),
                hero_image: None,
            }
        };

        let page1 = {
            let route = RouteInfo::new(
                "/blog/first-post/".to_string(),
                PathBuf::from("blog/first-post.md"),
                PathBuf::from("blog/first-post/index.html"),
                RouteKind::Page,
            )
            .unwrap();
            ProcessedPage {
                toc: Vec::new(),
                route,
                page: Page {
                    frontmatter: Frontmatter {
                        title: "First Post".to_string(),
                        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 15),
                        ..Default::default()
                    },
                    path: "/blog/first-post/".to_string(),
                    source: PathBuf::from("blog/first-post.md"),
                    raw_content: "First post content".to_string(),
                    content: None,
                },
                html_content: "<p>First post content</p>".to_string(),
                hero_image: None,
            }
        };

        let page2 = {
            let route = RouteInfo::new(
                "/blog/second-post/".to_string(),
                PathBuf::from("blog/second-post.md"),
                PathBuf::from("blog/second-post/index.html"),
                RouteKind::Page,
            )
            .unwrap();
            ProcessedPage {
                toc: Vec::new(),
                route,
                page: Page {
                    frontmatter: Frontmatter {
                        title: "Second Post".to_string(),
                        date: chrono::NaiveDate::from_ymd_opt(2024, 2, 20),
                        ..Default::default()
                    },
                    path: "/blog/second-post/".to_string(),
                    source: PathBuf::from("blog/second-post.md"),
                    raw_content: "Second post content".to_string(),
                    content: None,
                },
                html_content: "<p>Second post content</p>".to_string(),
                hero_image: None,
            }
        };

        let processed = vec![section_page, page1, page2];
        let result = render_pages(
            &processed,
            &tree_of(&processed),
            &templates,
            &test_site_context(),
            false,
        )
        .unwrap();

        let section_rendered = result
            .iter()
            .find(|r| r.route.path == "/blog/")
            .expect("Section page should be rendered");

        assert!(section_rendered.content.contains("Second Post"));
        assert!(section_rendered.content.contains("First Post"));

        let second_pos = section_rendered.content.find("Second Post").unwrap();
        let first_pos = section_rendered.content.find("First Post").unwrap();
        assert!(
            second_pos < first_pos,
            "Second Post (newer) should appear before First Post (older)"
        );
    }

    #[test]
    fn test_render_pages_section_without_child_pages() {
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

        let section_page = {
            let route = RouteInfo::new(
                "/empty/".to_string(),
                PathBuf::from("empty/_index.md"),
                PathBuf::from("empty/index.html"),
                RouteKind::Section,
            )
            .unwrap();
            ProcessedPage {
                toc: Vec::new(),
                route,
                page: Page {
                    frontmatter: Frontmatter {
                        title: "Empty Section".to_string(),
                        template: Some("section.html".to_string()),
                        ..Default::default()
                    },
                    path: "/empty/".to_string(),
                    source: PathBuf::from("empty/_index.md"),
                    raw_content: "Empty section".to_string(),
                    content: None,
                },
                html_content: "<p>Empty section</p>".to_string(),
                hero_image: None,
            }
        };

        let processed = vec![section_page];
        let result = render_pages(
            &processed,
            &tree_of(&processed),
            &templates,
            &test_site_context(),
            false,
        )
        .unwrap();

        let section_rendered = result
            .iter()
            .find(|r| r.route.path == "/empty/")
            .expect("Section page should be rendered");

        assert!(section_rendered.content.contains("<ul>"));
        assert!(section_rendered.content.contains("</ul>"));
        assert!(!section_rendered.content.contains("<li>"));
    }

    // ── Pagination tests ───────────────────────────────────────────────────────

    #[test]
    fn test_render_pages_with_pagination() {
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

        let section_page = {
            let route = RouteInfo::new(
                "/blog/".to_string(),
                PathBuf::from("blog/_index.md"),
                PathBuf::from("blog/index.html"),
                RouteKind::Section,
            )
            .unwrap();
            ProcessedPage {
                toc: Vec::new(),
                route,
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
                hero_image: None,
            }
        };

        let mut child_pages = Vec::new();
        for i in 1..=5 {
            let route = RouteInfo::new(
                format!("/blog/post-{}/", i),
                PathBuf::from(format!("blog/post-{}.md", i)),
                PathBuf::from(format!("blog/post-{}/index.html", i)),
                RouteKind::Page,
            )
            .unwrap();

            child_pages.push(ProcessedPage {
                route,
                page: Page {
                    frontmatter: Frontmatter {
                        title: format!("Post {}", i),
                        date: chrono::NaiveDate::from_ymd_opt(2024, 1, i as u32),
                        ..Default::default()
                    },
                    path: format!("/blog/post-{}/", i),
                    source: PathBuf::from(format!("blog/post-{}.md", i)),
                    raw_content: format!("Content {}", i),
                    content: None,
                },
                html_content: format!("<p>Content {}</p>", i),
                toc: Vec::new(),
                hero_image: None,
            });
        }

        let mut all_pages = vec![section_page];
        all_pages.extend(child_pages);

        let result = render_pages(
            &all_pages,
            &tree_of(&all_pages),
            &templates,
            &test_site_context(),
            false,
        )
        .unwrap();

        let section_pages: Vec<_> = result
            .iter()
            .filter(|r| r.route.path.starts_with("/blog/") && r.route.is_section())
            .collect();

        assert_eq!(section_pages.len(), 3, "Should have 3 paginated pages");
        assert!(section_pages.iter().any(|r| r.route.path == "/blog/"));
        assert!(
            section_pages
                .iter()
                .any(|r| r.route.path == "/blog/page/2/")
        );
        assert!(
            section_pages
                .iter()
                .any(|r| r.route.path == "/blog/page/3/")
        );

        let page1 = section_pages
            .iter()
            .find(|r| r.route.path == "/blog/")
            .unwrap();
        assert!(page1.content.contains("Page 1 of 3"));
        assert!(page1.content.contains("5 items"));
        assert!(!page1.content.contains("Prev"));
        assert!(page1.content.contains("Next"));

        let page3 = section_pages
            .iter()
            .find(|r| r.route.path == "/blog/page/3/")
            .unwrap();
        assert!(page3.content.contains("Page 3 of 3"));
        assert!(page3.content.contains("Prev"));
        assert!(!page3.content.contains("Next"));
    }

    #[test]
    fn test_render_pages_no_pagination_when_not_configured() {
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

        let section_page = {
            let route = RouteInfo::new(
                "/blog/".to_string(),
                PathBuf::from("blog/_index.md"),
                PathBuf::from("blog/index.html"),
                RouteKind::Section,
            )
            .unwrap();
            ProcessedPage {
                toc: Vec::new(),
                route,
                page: Page {
                    frontmatter: Frontmatter {
                        title: "Blog".to_string(),
                        template: Some("section.html".to_string()),
                        paginate_by: 0,
                        ..Default::default()
                    },
                    path: "/blog/".to_string(),
                    source: PathBuf::from("blog/_index.md"),
                    raw_content: "Blog".to_string(),
                    content: None,
                },
                html_content: "<p>Blog</p>".to_string(),
                hero_image: None,
            }
        };

        let child = {
            let route = RouteInfo::new(
                "/blog/post-1/".to_string(),
                PathBuf::from("blog/post-1.md"),
                PathBuf::from("blog/post-1/index.html"),
                RouteKind::Page,
            )
            .unwrap();
            ProcessedPage {
                toc: Vec::new(),
                route,
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
                hero_image: None,
            }
        };

        let all_pages = vec![section_page, child];
        let result = render_pages(
            &all_pages,
            &tree_of(&all_pages),
            &templates,
            &test_site_context(),
            false,
        )
        .unwrap();

        let section_pages: Vec<_> = result.iter().filter(|r| r.route.is_section()).collect();

        assert_eq!(section_pages.len(), 1);
        assert_eq!(section_pages[0].route.path, "/blog/");
        assert!(section_pages[0].content.contains("Post 1"));
    }

    // ── Extra variables tests ─────────────────────────────────────────────────

    #[test]
    fn test_render_pages_with_extra_variables() {
        let content = r#"
+++
title = "Styled Page"
tagline = "This is a test tagline."

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
            PathBuf::from("styled.md"),
            PathBuf::from("styled/index.html"),
            RouteKind::Page,
        )
        .unwrap();

        let processed = ProcessedPage {
            toc: Vec::new(),
            route,
            page,
            html_content: "<p>Content here.</p>".to_string(),
            hero_image: None,
        };

        let mut templates = TeraRenderer::new().unwrap();
        templates
            .register_template(
                "page.html",
                r#"<div class="{{ extra.css_class }}">
<img src="{{ extra.hero_image | safe }}" />
{% if extra.featured %}<span>Featured!</span>{% endif %}
<h1>{{ page.title }}</h1>
<h2>{{ page.tagline }}</h2>
</div>"#,
            )
            .unwrap();

        let processed = [processed];
        let rendered = render_pages(
            &processed,
            &tree_of(&processed),
            &templates,
            &test_site_context(),
            false,
        )
        .unwrap();

        assert_eq!(rendered.len(), 1);
        let html = &rendered[0].content;
        assert!(
            html.contains("dark-theme"),
            "Should contain css_class extra variable"
        );
        assert!(
            html.contains("/images/hero.jpg"),
            "Should contain hero_image extra variable"
        );
        assert!(
            html.contains("This is a test tagline."),
            "Should contain tagline variable"
        );
        assert!(
            html.contains("Featured!"),
            "Should contain featured extra variable"
        );
    }

    // ── Membership tests (#70) ────────────────────────────────────────────────

    fn section_page(path: &str, file: &str, fm: Frontmatter) -> ProcessedPage {
        let output = if path == "/" {
            "index.html".to_string()
        } else {
            format!("{}/index.html", path.trim_matches('/'))
        };
        ProcessedPage {
            toc: Vec::new(),
            route: RouteInfo::new(
                path.to_string(),
                PathBuf::from(file),
                PathBuf::from(output),
                RouteKind::Section,
            )
            .unwrap(),
            page: Page {
                frontmatter: fm,
                path: path.to_string(),
                source: PathBuf::from(file),
                raw_content: String::new(),
                content: None,
            },
            html_content: String::new(),
            hero_image: None,
        }
    }

    fn plain_page(path: &str, file: &str, title: &str) -> ProcessedPage {
        ProcessedPage {
            toc: Vec::new(),
            route: RouteInfo::new(
                path.to_string(),
                PathBuf::from(file),
                PathBuf::from(format!("{}/index.html", path.trim_matches('/'))),
                RouteKind::Page,
            )
            .unwrap(),
            page: Page {
                frontmatter: Frontmatter {
                    title: title.to_string(),
                    ..Default::default()
                },
                path: path.to_string(),
                source: PathBuf::from(file),
                raw_content: String::new(),
                content: None,
            },
            html_content: String::new(),
            hero_image: None,
        }
    }

    fn listing_template() -> TeraRenderer {
        let mut templates = TeraRenderer::new().unwrap();
        templates
            .register_template("page.html", "{{ page.title }}")
            .unwrap();
        templates
            .register_template(
                "section.html",
                "{% for p in section.pages %}[{{ p.title }}]{% endfor %}",
            )
            .unwrap();
        templates
    }

    fn listed(result: &[RenderedPage], path: &str) -> String {
        result
            .iter()
            .find(|r| r.route.path == path)
            .unwrap()
            .content
            .clone()
    }

    #[test]
    fn test_section_lists_direct_children_only() {
        let processed = vec![
            section_page("/", "_index.md", Frontmatter::default()),
            section_page("/blog/", "blog/_index.md", Frontmatter::default()),
            plain_page("/about/", "about.md", "About"),
            plain_page("/blog/top/", "blog/top.md", "Top"),
            plain_page("/blog/2026/nested/", "blog/2026/nested.md", "Nested"),
        ];
        let result = render_pages(
            &processed,
            &tree_of(&processed),
            &listing_template(),
            &test_site_context(),
            false,
        )
        .unwrap();

        assert_eq!(listed(&result, "/"), "[About]");
        assert_eq!(listed(&result, "/blog/"), "[Top]");
    }

    #[test]
    fn test_pages_from_adds_donor_sections_direct_pages() {
        let root_meta = Frontmatter {
            pages_from: vec!["blog".into(), "docs".into(), "missing".into()],
            sort_by: SortBy::Title,
            ..Default::default()
        };
        let processed = vec![
            section_page("/", "_index.md", root_meta),
            section_page("/blog/", "blog/_index.md", Frontmatter::default()),
            plain_page("/about/", "about.md", "About"),
            plain_page("/blog/top/", "blog/top.md", "Top"),
            plain_page("/blog/2026/nested/", "blog/2026/nested.md", "Nested"),
            plain_page("/docs/guide/", "docs/guide.md", "Guide"),
        ];
        let result = render_pages(
            &processed,
            &tree_of(&processed),
            &listing_template(),
            &test_site_context(),
            false,
        )
        .unwrap();

        // Own page + donors' direct pages (not blog/2026), sorted by the
        // receiver's sort_by; the unknown donor is ignored with a warning.
        assert_eq!(listed(&result, "/"), "[About][Guide][Top]");
        assert_eq!(listed(&result, "/blog/"), "[Top]");
    }
}
