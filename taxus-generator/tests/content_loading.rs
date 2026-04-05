//! Integration tests for content loading.

use std::path::PathBuf;
use taxus_lib::content::{ContentSource, FilesystemContentSource, Frontmatter, Page, Section};
use taxus_lib::error::{ContentError, GeneratorError};

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
    assert_eq!(page.path, "/about/");
    assert!(page.raw_content.contains("# About"));
}

#[test]
fn test_load_home_page() {
    let result = Page::from_file("tests/fixtures/content_site/content/_index.md");

    assert!(result.is_ok());
    let page = result.unwrap();

    assert_eq!(page.path, "/");
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

#[test]
fn test_section_from_dir() {
    let result = Section::from_dir("tests/fixtures/content_site/content/blog");

    assert!(result.is_ok());
    let section = result.unwrap();

    assert_eq!(section.path, "/blog/");
    assert_eq!(section.frontmatter.title, "Blog");
    assert_eq!(
        section.frontmatter.template,
        Some("section.html".to_string())
    );
}

#[test]
fn test_page_draft_status() {
    let draft = Page::from_file("tests/fixtures/content_site/content/blog/draft-post.md").unwrap();
    let published =
        Page::from_file("tests/fixtures/content_site/content/blog/first-post.md").unwrap();

    assert!(draft.is_draft());
    assert!(!published.is_draft());
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
