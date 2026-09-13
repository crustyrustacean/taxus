// taxus-generator/src/build/pipeline/pages.rs

//! Stage 6 (analyse, emit): build template contexts and render every document.
//!
//! The analyse half is here: a section's listing is
//! `taxus_domain::derivation::aggregate` sorted by
//! `taxus_domain::tree::sort_pages`, and pagination slices that listing.
//! The emit half builds a `TemplateContext` per document and runs its
//! template. This module also fills what the `get_section` and
//! `get_page` template functions resolve to. See the book's
//! [Derivations](https://crustyrustacean.github.io/taxus/theory/derivations.html) and
//! [Worked Example](https://crustyrustacean.github.io/taxus/theory/worked-example.html) chapters.

use crate::build::{ProcessedPage, RenderedPage};
use crate::error::Result;
use crate::routes::RouteInfo;
use crate::templates::{
    HeroContext, PageContext, PaginationContext, SectionContext, SiteContext, SubsectionContext,
    TemplateContext, TemplateRenderer, TeraRenderer, compute_permalink,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use taxus_domain::{NodePath, SectionNode, SiteTree, UrlPath, derivation};
use tracing::{debug, debug_span, info, warn};

/// Build a `PageContext` from a `ProcessedPage`.
///
/// The page's path is its served URL (`effective_url_path`, derived from
/// the tree); the permalink is that path on the site's base URL.
fn page_context_from(processed: &ProcessedPage, base_url: &str) -> PageContext {
    let url_path = processed.effective_url_path();
    let permalink = compute_permalink(base_url, &url_path);
    let hero = processed.hero_image.as_ref().and_then(|img| {
        img.fallback_src().map(|src| HeroContext {
            src,
            srcset: img.srcset(),
            width: img.meta.original_width,
            height: img.meta.original_height,
            alt: img.meta.alt.clone(),
            mime_type: img.mime_type(),
        })
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

/// The pages a section lists: the section node's `pages`, plus the direct
/// pages of every section named in its `pages_from` frontmatter
/// ([`derivation::aggregate`]). Deeper descendants are not listed, so the
/// root lists the site only if its `_index.md` declares `pages_from` (#70).
/// Pages the build skipped (drafts) are dropped. Ordering is the domain's
/// [`taxus_domain::tree::sort_pages`] with the section's `sort_by`: date
/// newest first with undated pages last, title case-insensitive, weight
/// lowest first; ties keep tree (slug) order.
fn collect_child_pages(
    node: &SectionNode,
    tree: &SiteTree,
    processed_by_file: &HashMap<&Path, &ProcessedPage>,
    base_url: &str,
) -> Vec<PageContext> {
    let mut donors = Vec::new();
    for raw in &node.meta.pages_from {
        match NodePath::parse(raw) {
            Ok(path) if tree.get_section(&path).is_some() => donors.push(path),
            Ok(_) => warn!(
                section = %node.path,
                pages_from = %raw,
                "pages_from names a section that does not exist; ignoring it"
            ),
            Err(e) => warn!(
                section = %node.path,
                pages_from = %raw,
                error = %e,
                "pages_from entry is not a valid section path; ignoring it"
            ),
        }
    }

    let mut pages = derivation::aggregate(node, tree, &donors);
    taxus_domain::tree::sort_pages(&mut pages, node.meta.sort_by);
    pages
        .iter()
        .filter_map(|node| processed_by_file.get(node.content_file.as_path()))
        .map(|p| page_context_from(p, base_url))
        .collect()
}

/// A child section as templates see it on `section.subsections`.
fn subsection_context(node: &SectionNode, base_url: &str) -> SubsectionContext {
    let path = UrlPath::from_node_path(&node.path).to_string();
    SubsectionContext {
        title: node.meta.title.clone(),
        description: node.meta.description.clone(),
        permalink: compute_permalink(base_url, &path),
        path,
    }
}

/// The template view of a section node: what `section` holds while the
/// section's own index renders, and what `get_section` returns (#69).
///
/// `content` and `toc` come from the rendered `_index.md` when there is
/// one (a directory without an index file is still a section, with default
/// frontmatter and no content); `pages` is [`collect_child_pages`];
/// `subsections` are the direct child sections in tree order. `pagination`
/// is `None`: slicing belongs to the owning section's render, not to the
/// view.
fn section_context_for(
    node: &SectionNode,
    tree: &SiteTree,
    processed_by_file: &HashMap<&Path, &ProcessedPage>,
    base_url: &str,
) -> SectionContext {
    let rendered = node
        .content_file
        .as_deref()
        .and_then(|file| processed_by_file.get(file))
        .copied();
    let path = UrlPath::from_node_path(&node.path).to_string();
    SectionContext {
        title: node.meta.title.clone(),
        description: node.meta.description.clone(),
        permalink: compute_permalink(base_url, &path),
        path,
        content: rendered.map(|p| p.html_content.clone()),
        toc: rendered.map(|p| p.toc.clone()).unwrap_or_default(),
        pages: collect_child_pages(node, tree, processed_by_file, base_url),
        pagination: None,
        subsections: node
            .subsections
            .iter()
            .map(|sub| subsection_context(sub, base_url))
            .collect(),
    }
}

/// Sections keyed for `get_section`, see [`site_lookup`].
type SectionLookup = Vec<(String, SectionContext)>;
/// Pages keyed for `get_page`, see [`site_lookup`].
type PageLookup = Vec<(String, PageContext)>;

/// Every section and page as `get_section` / `get_page` fetch them.
///
/// Sections are keyed by tree path (`blog`; the root is `""`). Pages are
/// keyed by tree path (`blog/my-post`) and by content file
/// (`blog/2026-04-06-my-post.md`); pages the build skipped (drafts) are
/// absent.
fn site_lookup(
    tree: &SiteTree,
    processed_by_file: &HashMap<&Path, &ProcessedPage>,
    base_url: &str,
) -> (SectionLookup, PageLookup) {
    fn walk(
        node: &SectionNode,
        tree: &SiteTree,
        processed_by_file: &HashMap<&Path, &ProcessedPage>,
        base_url: &str,
        sections: &mut SectionLookup,
        pages: &mut PageLookup,
    ) {
        sections.push((
            node.path.to_string(),
            section_context_for(node, tree, processed_by_file, base_url),
        ));
        for page in &node.pages {
            if let Some(rendered) = processed_by_file.get(page.content_file.as_path()) {
                let context = page_context_from(rendered, base_url);
                pages.push((
                    page.content_file.to_string_lossy().replace('\\', "/"),
                    context.clone(),
                ));
                pages.push((page.path.to_string(), context));
            }
        }
        for sub in &node.subsections {
            walk(sub, tree, processed_by_file, base_url, sections, pages);
        }
    }

    let mut sections = Vec::new();
    let mut pages = Vec::new();
    walk(
        &tree.root,
        tree,
        processed_by_file,
        base_url,
        &mut sections,
        &mut pages,
    );
    (sections, pages)
}

/// Render a paginated section.
///
/// Handles the entire pagination loop: slices the section view's pages per
/// pagination page, builds `PaginationContext`, renders each page with the
/// template, and collects results. Uses `page_context_from` internally,
/// overriding `path` and `permalink` for each pagination page. First page
/// outputs to the section's normal path; subsequent pages go to `/page/N/`
/// subdirectories.
fn render_paginated_section(
    processed_page: &ProcessedPage,
    mut section: SectionContext,
    templates: &TeraRenderer,
    site_context: &SiteContext,
    url_path: &str,
) -> Result<Vec<RenderedPage>> {
    let mut rendered = Vec::new();
    let paginate_by = processed_page.page.frontmatter.paginate_by;
    let child_pages = std::mem::take(&mut section.pages);
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
            path: page_url(page_num),
            permalink: compute_permalink(&site_context.base_url, &page_url(page_num)),
            pages: slice,
            pagination: Some(pagination_context),
            ..section.clone()
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

        rendered.push(RenderedPage {
            route,
            content,
            hero_image: None,
        });
    }

    Ok(rendered)
}

/// Render pages using templates.
///
/// Iterates through processed pages, builds template contexts, and renders each
/// to HTML. For sections, the listed pages come from `tree` (see
/// `collect_child_pages`); paginated sections are dispatched to
/// `render_paginated_section`. Regular pages and non-paginated sections are
/// rendered directly.
pub fn render_pages(
    processed: &[ProcessedPage],
    tree: &SiteTree,
    templates: &TeraRenderer,
    site_context: &SiteContext,
) -> Result<Vec<RenderedPage>> {
    let span = debug_span!("render_pages", pages = processed.len());
    let _enter = span.enter();

    // Tree nodes are joined back to their processed pages by content file:
    // storage identity, shared by both, and absent for skipped drafts.
    let processed_by_file: HashMap<&Path, &ProcessedPage> = processed
        .iter()
        .map(|p| (p.route.content_file.as_path(), p))
        .collect();

    // What templates fetch with get_section / get_page (#69): every
    // section and page of the tree, as rendered in this build.
    let (sections, pages) = site_lookup(tree, &processed_by_file, &site_context.base_url);
    templates.set_site_lookup(sections, pages);

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
            let section_context = NodePath::parse(&processed_page.route.path)
                .ok()
                .and_then(|path| tree.get_section(&path))
                .map(|node| {
                    section_context_for(node, tree, &processed_by_file, &site_context.base_url)
                })
                .unwrap_or_else(|| {
                    debug!(
                        path = %processed_page.route.path,
                        "Section route has no tree node; listing nothing"
                    );
                    SectionContext {
                        title: processed_page.page.frontmatter.title.clone(),
                        description: processed_page.page.frontmatter.description.clone(),
                        path: url_path.clone(),
                        permalink: compute_permalink(&site_context.base_url, &url_path),
                        content: Some(processed_page.html_content.clone()),
                        toc: processed_page.toc.clone(),
                        pages: Vec::new(),
                        pagination: None,
                        subsections: Vec::new(),
                    }
                });

            let paginate_by = processed_page.page.frontmatter.paginate_by;
            if paginate_by > 0 && !section_context.pages.is_empty() {
                rendered.extend(render_paginated_section(
                    processed_page,
                    section_context,
                    templates,
                    site_context,
                    &url_path,
                )?);
                continue;
            }

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

        let route = RouteInfo::new(
            url_path,
            processed_page.route.content_file.clone(),
            processed_page.route.output_file.clone(),
            processed_page.route.kind,
        )?;

        rendered.push(RenderedPage {
            route,
            content,
            hero_image: processed_page.hero_image.clone(),
        });
    }

    info!("Rendered {} pages", rendered.len());
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::SortBy;
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

    /// A `slug` replaces the last segment of the page's tree path; the page
    /// stays in its section. Discovery already places it there, so the
    /// route handed to rendering is the served URL and nothing here
    /// second-guesses it.
    #[test]
    fn test_render_pages_with_custom_slug() {
        let content = r#"
+++
title = "Test Post"
slug = "custom-url"
+++
This is the content.
"#;
        let page = Page::from_str(content.trim_start(), "blog/original-filename.md").unwrap();

        // What `RouteRegistry::from_tree` derives for this page.
        let route = RouteInfo::new(
            "/blog/custom-url/".to_string(),
            PathBuf::from("blog/original-filename.md"),
            PathBuf::from("blog/custom-url/index.html"),
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
        )
        .unwrap();

        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].route.path, "/blog/custom-url/");
        assert_eq!(
            rendered[0].route.output_file,
            PathBuf::from("blog/custom-url/index.html")
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
                    raw_content: "Blog index".to_string(),
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
                    raw_content: "First post content".to_string(),
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
                    raw_content: "Second post content".to_string(),
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
                    raw_content: "Empty section".to_string(),
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
                    raw_content: "Blog index".to_string(),
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
                    raw_content: format!("Content {}", i),
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
                    raw_content: "Blog".to_string(),
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
                    raw_content: "Content".to_string(),
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
                raw_content: String::new(),
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
                raw_content: String::new(),
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
        )
        .unwrap();

        // Own page + donors' direct pages (not blog/2026), sorted by the
        // receiver's sort_by; the unknown donor is ignored with a warning.
        assert_eq!(listed(&result, "/"), "[About][Guide][Top]");
        assert_eq!(listed(&result, "/blog/"), "[Top]");
    }

    #[test]
    fn test_section_subsections_and_tree_functions() {
        let processed = vec![
            section_page("/", "_index.md", Frontmatter::default()),
            section_page("/blog/", "blog/_index.md", Frontmatter::default()),
            plain_page("/about/", "about.md", "About"),
            plain_page("/blog/top/", "blog/top.md", "Top"),
            plain_page("/docs/guide/", "docs/guide.md", "Guide"),
        ];
        let mut templates = TeraRenderer::new().unwrap();
        templates
            .register_template("page.html", "{{ page.title }}")
            .unwrap();
        templates
            .register_template(
                "section.html",
                concat!(
                    "{% for s in section.subsections %}<{{ s.path }}>{% endfor %}",
                    "{% set blog = get_section(path=\"blog/_index.md\") %}",
                    "{% for p in blog.pages %}[{{ p.title }}]{% endfor %}",
                    "{% set docs = get_section(path=\"docs\") %}",
                    "{% for p in docs.pages %}[{{ p.title }}]{% endfor %}",
                    "{% set about = get_page(path=\"about.md\") %}({{ about.path }})",
                ),
            )
            .unwrap();
        let result = render_pages(
            &processed,
            &tree_of(&processed),
            &templates,
            &test_site_context(),
        )
        .unwrap();
        // `docs/` has no _index.md and is still a section the tree knows.
        assert_eq!(
            listed(&result, "/"),
            "</blog/></docs/>[Top][Guide](/about/)"
        );
        assert_eq!(listed(&result, "/blog/"), "[Top][Guide](/about/)");
    }

    fn dated_page(path: &str, file: &str, title: &str, date: Option<&str>) -> ProcessedPage {
        let mut page = plain_page(path, file, title);
        page.page.frontmatter.date =
            date.map(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap());
        page
    }

    #[test]
    fn test_date_sort_is_newest_first_with_undated_last() {
        let processed = vec![
            section_page("/notes/", "notes/_index.md", Frontmatter::default()),
            dated_page("/notes/a/", "notes/a.md", "Undated", None),
            dated_page("/notes/b/", "notes/b.md", "Older", Some("2026-01-01")),
            dated_page("/notes/c/", "notes/c.md", "Newer", Some("2026-03-01")),
        ];
        let result = render_pages(
            &processed,
            &tree_of(&processed),
            &listing_template(),
            &test_site_context(),
        )
        .unwrap();
        assert_eq!(listed(&result, "/notes/"), "[Newer][Older][Undated]");
    }

    #[test]
    fn test_title_sort_is_case_insensitive() {
        let meta = Frontmatter {
            sort_by: SortBy::Title,
            ..Default::default()
        };
        let processed = vec![
            section_page("/glossary/", "glossary/_index.md", meta),
            plain_page("/glossary/c/", "glossary/c.md", "cherry"),
            plain_page("/glossary/b/", "glossary/b.md", "Banana"),
            plain_page("/glossary/a/", "glossary/a.md", "apple"),
        ];
        let result = render_pages(
            &processed,
            &tree_of(&processed),
            &listing_template(),
            &test_site_context(),
        )
        .unwrap();
        // Byte order would put "Banana" first.
        assert_eq!(listed(&result, "/glossary/"), "[apple][Banana][cherry]");
    }
}
