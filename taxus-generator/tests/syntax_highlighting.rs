//! Integration tests for syntax highlighting.

use std::path::Path;
use taxus_lib::build::SiteBuilder;
use taxus_lib::config::SiteConfig;
use tempfile::TempDir;

fn build_highlight_site() -> (TempDir, String) {
    let fixture_dir = Path::new("tests/fixtures/highlight_site");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().to_path_buf();

    let mut config = SiteConfig::from_dir(fixture_dir).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();

    let builder = SiteBuilder::new(config);
    builder.build().expect("Build failed");

    let html = std::fs::read_to_string(output_dir.join("index.html"))
        .expect("Failed to read output");

    (temp_dir, html)
}

#[test]
fn test_highlighted_rust_block_has_spans() {
    let (_dir, html) = build_highlight_site();

    assert!(
        html.contains("hl-keyword"),
        "Rust code block should contain highlighted keywords"
    );
    assert!(
        html.contains("hl-function"),
        "Rust code block should contain highlighted functions"
    );
    assert!(
        html.contains("hl-type"),
        "Rust code block should contain highlighted types"
    );
    assert!(
        html.contains("hl-string"),
        "Rust code block should contain highlighted strings"
    );
}

#[test]
fn test_highlighted_rust_block_has_wrapper() {
    let (_dir, html) = build_highlight_site();

    assert!(
        html.contains("<pre class=\"highlight\">"),
        "Highlighted code should be wrapped in pre.highlight"
    );
    assert!(
        html.contains("language-rust"),
        "Highlighted code should have language-rust class"
    );
}

#[test]
fn test_unknown_language_falls_back() {
    let (_dir, html) = build_highlight_site();

    assert!(
        html.contains("language-brainfuck"),
        "Unknown language should still have language class"
    );
    // Should NOT have the highlight wrapper
    assert!(
        html.contains("<pre><code class=\"language-brainfuck\">"),
        "Unknown language should use plain pre/code without highlight class"
    );
}

#[test]
fn test_no_language_plain_block() {
    let (_dir, html) = build_highlight_site();

    assert!(
        html.contains("no language specified"),
        "Plain code block content should be present"
    );
}

#[test]
fn test_html_is_escaped_in_code() {
    let (_dir, html) = build_highlight_site();

    // HashMap<&str, i32> should have escaped angle brackets
    assert!(
        html.contains("&amp;") || html.contains("&lt;"),
        "HTML special characters in code should be escaped"
    );
    assert!(
        !html.contains("<&str"),
        "Raw angle brackets around types should not appear"
    );
}

#[test]
fn test_non_code_content_renders_normally() {
    let (_dir, html) = build_highlight_site();

    assert!(
        html.contains("<h1>Syntax Highlighting Test</h1>"),
        "Non-code content should render as normal HTML"
    );
}

#[test]
fn test_highlighting_disabled_produces_plain_blocks() {
    let fixture_dir = Path::new("tests/fixtures/highlight_site");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().to_path_buf();

    let mut config = SiteConfig::from_dir(fixture_dir).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();
    config.highlight.enabled = false;

    let builder = SiteBuilder::new(config);
    builder.build().expect("Build failed");

    let html = std::fs::read_to_string(output_dir.join("index.html"))
        .expect("Failed to read output");

    assert!(
        !html.contains("hl-keyword"),
        "Disabled highlighting should not produce highlight spans"
    );
    assert!(
        !html.contains("<pre class=\"highlight\">"),
        "Disabled highlighting should not produce highlight wrapper"
    );
}

#[test]
fn test_custom_class_prefix() {
    let fixture_dir = Path::new("tests/fixtures/highlight_site");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().to_path_buf();

    let mut config = SiteConfig::from_dir(fixture_dir).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();
    config.highlight.class_prefix = "syntax-".to_string();

    let builder = SiteBuilder::new(config);
    builder.build().expect("Build failed");

    let html = std::fs::read_to_string(output_dir.join("index.html"))
        .expect("Failed to read output");

    assert!(
        html.contains("syntax-keyword"),
        "Custom prefix should be used in span classes"
    );
    assert!(
        !html.contains("hl-keyword"),
        "Default prefix should not appear with custom prefix"
    );
}