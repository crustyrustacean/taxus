//! Integration tests for content loading.

use std::path::PathBuf;
use taxus_domain::NodePath;
use taxus_lib::content::{ContentSource, FilesystemContentSource, Frontmatter, Page};
use taxus_lib::error::{ContentError, GeneratorError};
use taxus_lib::routes::RouteDiscovery;

#[test]
fn test_load_page_from_file() {
    let result = Page::from_file("tests/fixtures/content_site/content/about.md");

    assert!(result.is_ok());
    let page = result.unwrap();

    assert_eq!(page.frontmatter.title, "About");
    assert_eq!(
        page.frontmatter.description,
        Some("About this site".to_string())
    );
    assert!(page.raw_content.contains("# About"));
}

#[test]
fn test_load_home_page() {
    let result = Page::from_file("tests/fixtures/content_site/content/_index.md");

    assert!(result.is_ok());
    let page = result.unwrap();

    assert_eq!(page.frontmatter.title, "Home");
}

#[test]
fn test_load_page_not_found() {
    let result = Page::from_file("tests/fixtures/content_site/content/nonexistent.md");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        GeneratorError::Content(inner) if matches!(*inner, ContentError::Io { .. })
    ));
}

#[test]
fn test_filesystem_content_source_list() {
    let source = FilesystemContentSource::new("tests/fixtures/content_site/content");

    let result = source.list();
    assert!(result.is_ok());

    let files = result.unwrap();
    assert!(!files.is_empty());

    // Should find all .md files
    assert!(files.iter().any(|f| f.ends_with("_index.md")));
    assert!(files.iter().any(|f| f.ends_with("about.md")));
}

#[test]
fn test_filesystem_content_source_load() {
    let source = FilesystemContentSource::new("tests/fixtures/content_site/content");

    let result = source.load(&PathBuf::from("about.md"));
    assert!(result.is_ok());

    let content = result.unwrap();
    assert!(content.contains("title = \"About\""));
}

#[test]
fn test_filesystem_content_source_exists() {
    let source = FilesystemContentSource::new("tests/fixtures/content_site/content");

    assert!(source.exists(&PathBuf::from("about.md")));
    assert!(!source.exists(&PathBuf::from("nonexistent.md")));
}

/// Sections are Site Tree nodes: the blog directory's `_index.md` becomes
/// a `SectionNode` with its frontmatter, and its pages hang off it.
#[test]
fn test_section_is_a_tree_node() {
    let tree = RouteDiscovery::new("tests/fixtures/content_site/content")
        .discover_tree()
        .unwrap();
    let blog = tree
        .get_section(&NodePath::parse("blog").unwrap())
        .expect("blog section exists");

    assert_eq!(
        taxus_domain::UrlPath::from_node_path(&blog.path).as_str(),
        "/blog/"
    );
    assert_eq!(blog.meta.title, "Blog");
    assert_eq!(blog.meta.template, Some("section.html".to_string()));
    assert_eq!(
        blog.content_file.as_deref(),
        Some(std::path::Path::new("blog/_index.md"))
    );
    assert_eq!(blog.pages.len(), 2);
}

#[test]
fn test_page_draft_status() {
    let draft = Page::from_file("tests/fixtures/content_site/content/blog/draft-post.md").unwrap();
    let published =
        Page::from_file("tests/fixtures/content_site/content/blog/first-post.md").unwrap();

    assert!(draft.is_draft());
    assert!(!published.is_draft());
}

/// #55: the draft count is observed where skips happen, not inferred by
/// subtraction. `content_site` carries exactly one draft; the count must
/// equal it whether or not other documents would have been skipped, and
/// `--include-drafts` must report zero skips.
#[test]
fn test_process_content_reports_observed_draft_count() {
    use taxus_lib::config::SiteConfig;
    use taxus_lib::routes::RouteRegistry;

    let config = SiteConfig::from_dir("tests/fixtures/content_site").unwrap();

    let content_dir = PathBuf::from("tests/fixtures/content_site/content");
    let tree = RouteDiscovery::new(&content_dir).discover_tree().unwrap();
    let registry = RouteRegistry::from_tree(&tree);

    let (processed, skipped) =
        taxus_lib::build::pipeline::process_content(&tree, &registry, &config, false, None)
            .unwrap();

    // The site's documents are three pages (about, draft-post,
    // first-post) plus two section indexes (root, blog); the one draft
    // is skipped, leaving four processed.
    assert_eq!(skipped, 1, "exactly the one draft is counted as skipped");
    assert_eq!(processed.len(), 4);

    let (processed, skipped) =
        taxus_lib::build::pipeline::process_content(&tree, &registry, &config, true, None).unwrap();
    assert_eq!(skipped, 0, "including drafts skips nothing");
    assert_eq!(processed.len(), 5);
}

#[test]
fn test_frontmatter_with_extra() {
    // Test that extra metadata can be parsed
    let content = r#"
title = "Test"
description = "Test page"

[extra]
author = "John Doe"
tags = ["rust", "web"]
"#;

    let fm: Frontmatter = toml::from_str(content).unwrap();
    assert_eq!(fm.title, "Test");
    assert!(fm.extra.is_some());
}
