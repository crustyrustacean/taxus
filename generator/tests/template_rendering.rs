//! Integration tests for template rendering.

use std::collections::HashMap;
use std::path::PathBuf;
use yew_ssg_lib::error::TemplateError;
use yew_ssg_lib::templates::{
    PageContext, SectionContext, SiteContext, TemplateContext, TemplateRenderer, TeraRenderer,
};

fn create_test_site_context() -> SiteContext {
    SiteContext {
        name: "Integration Test Site".to_string(),
        base_url: "https://test.example.com".to_string(),
        description: Some("Integration test site".to_string()),
        author: Some("Test Author".to_string()),
    }
}

fn create_test_page_context() -> PageContext {
    PageContext {
        title: "Test Page".to_string(),
        description: Some("A test page".to_string()),
        path: "/test/".to_string(),
        permalink: "https://test.example.com/test/".to_string(),
        content: "<p>This is test content.</p>".to_string(),
        raw_content: "This is test content.".to_string(),
        date: Some("2024-01-15".to_string()),
        draft: false,
        summary: "A test page summary".to_string(),
        word_count: 4,
        reading_time: 1,
        tags: vec!["rust".to_string()],
        categories: vec!["programming".to_string()],
        series: None,
    }
}

fn create_test_section_context() -> SectionContext {
    SectionContext {
        title: "Blog".to_string(),
        path: "/blog/".to_string(),
        pages: vec![
            PageContext {
                title: "First Post".to_string(),
                description: Some("My first post".to_string()),
                path: "/blog/first/".to_string(),
                permalink: "https://test.example.com/blog/first/".to_string(),
                content: "<p>First post content</p>".to_string(),
                raw_content: "First post content".to_string(),
                date: Some("2024-01-10".to_string()),
                draft: false,
                summary: "First post summary".to_string(),
                word_count: 3,
                reading_time: 1,
                tags: vec!["rust".to_string(), "tutorial".to_string()],
                categories: vec!["programming".to_string()],
                series: Some("Learning Rust".to_string()),
            },
            PageContext {
                title: "Second Post".to_string(),
                description: Some("My second post".to_string()),
                path: "/blog/second/".to_string(),
                permalink: "https://test.example.com/blog/second/".to_string(),
                content: "<p>Second post content</p>".to_string(),
                raw_content: "Second post content".to_string(),
                date: Some("2024-01-20".to_string()),
                draft: false,
                summary: "Second post summary".to_string(),
                word_count: 3,
                reading_time: 1,
                tags: vec!["rust".to_string(), "advanced".to_string()],
                categories: vec!["programming".to_string()],
                series: Some("Learning Rust".to_string()),
            },
        ],
        pagination: None,
    }
}

#[test]
fn test_load_templates_from_directory() {
    let result = TeraRenderer::from_dir("tests/fixtures/template_site/templates");

    assert!(result.is_ok());
    let renderer = result.unwrap();
    assert!(renderer.has_template("base.html"));
    assert!(renderer.has_template("page.html"));
    assert!(renderer.has_template("section.html"));
}

#[test]
fn test_render_page_template() {
    let mut renderer = TeraRenderer::new().unwrap();

    // Register a simple page template
    renderer
        .register_template(
            "page.html",
            r#"<article><h1>{{ page.title }}</h1>{{ page.content | safe }}</article>"#,
        )
        .unwrap();

    let ctx =
        TemplateContext::new(create_test_site_context()).with_page(create_test_page_context());

    let result = renderer.render("page.html", &ctx);
    assert!(result.is_ok());

    let html = result.unwrap();
    assert!(html.contains("<h1>Test Page</h1>"));
    assert!(html.contains("<p>This is test content.</p>"));
}

#[test]
fn test_render_section_template() {
    let mut renderer = TeraRenderer::new().unwrap();

    // Register a section template
    renderer
        .register_template(
            "section.html",
            r#"<section><h1>{{ section.title }}</h1>{% for p in section.pages %}<a href="{{ p.path }}">{{ p.title }}</a>{% endfor %}</section>"#,
        )
        .unwrap();

    let ctx = TemplateContext::new(create_test_site_context())
        .with_section(create_test_section_context());

    let result = renderer.render("section.html", &ctx);
    assert!(result.is_ok());

    let html = result.unwrap();
    assert!(html.contains("<h1>Blog</h1>"));
    assert!(html.contains("First Post"));
    assert!(html.contains("Second Post"));
}

#[test]
fn test_template_with_site_variables() {
    let mut renderer = TeraRenderer::new().unwrap();

    renderer
        .register_template(
            "base.html",
            r#"<html><head><title>{{ site.name }}</title></head><body></body></html>"#,
        )
        .unwrap();

    let ctx = TemplateContext::new(create_test_site_context());
    let result = renderer.render("base.html", &ctx);

    assert!(result.is_ok());
    let html = result.unwrap();
    assert!(html.contains("Integration Test Site"));
}

#[test]
fn test_template_inheritance() {
    let mut renderer = TeraRenderer::new().unwrap();

    // Register base template
    renderer
        .register_template(
            "base.html",
            r#"<html><head>{% block title %}{% endblock %}</head><body>{% block content %}{% endblock %}</body></html>"#,
        )
        .unwrap();

    // Register child template
    renderer
        .register_template(
            "page.html",
            r#"{% extends "base.html" %}{% block title %}{{ page.title }}{% endblock %}{% block content %}<p>{{ page.content | safe }}</p>{% endblock %}"#,
        )
        .unwrap();

    let ctx =
        TemplateContext::new(create_test_site_context()).with_page(create_test_page_context());

    let result = renderer.render("page.html", &ctx);
    assert!(result.is_ok());

    let html = result.unwrap();
    assert!(html.contains("<head>Test Page</head>"));
}

#[test]
fn test_missing_template_error() {
    let renderer = TeraRenderer::new().unwrap();
    let ctx = TemplateContext::new(create_test_site_context());

    let result = renderer.render("nonexistent.html", &ctx);
    assert!(result.is_err());

    match result.unwrap_err() {
        TemplateError::NotFound(name) => assert_eq!(name, "nonexistent.html"),
        e => panic!("Expected NotFound error, got: {}", e),
    }
}

#[test]
fn test_invalid_template_syntax_error() {
    let mut renderer = TeraRenderer::new().unwrap();
    let result = renderer.register_template("bad.html", "{{ unclosed");

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TemplateError::Syntax { .. }));
}

#[test]
fn test_template_with_extra_variables() {
    let mut renderer = TeraRenderer::new().unwrap();

    renderer
        .register_template("extra.html", r#"<div>{{ extra.custom_var }}</div>"#)
        .unwrap();

    let mut extra = HashMap::new();
    extra.insert("custom_var".to_string(), serde_json::json!("Custom Value"));

    let ctx = TemplateContext::new(create_test_site_context()).with_extra(extra);

    let result = renderer.render("extra.html", &ctx);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Custom Value"));
}

#[test]
fn test_load_templates_missing_directory() {
    let result = TeraRenderer::from_dir("tests/fixtures/nonexistent_templates");

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TemplateError::DirNotFound(_)));
}

#[test]
fn test_render_with_fixture_templates() {
    let renderer = TeraRenderer::from_dir("tests/fixtures/template_site/templates").unwrap();

    let ctx =
        TemplateContext::new(create_test_site_context()).with_page(create_test_page_context());

    // Test page template
    let result = renderer.render("page.html", &ctx);
    assert!(result.is_ok());

    let html = result.unwrap();
    assert!(html.contains("Test Page"));
    assert!(html.contains("Integration Test Site"));
    assert!(html.contains("<article>"));
}

#[test]
fn test_render_section_with_fixture_templates() {
    let renderer = TeraRenderer::from_dir("tests/fixtures/template_site/templates").unwrap();

    let ctx = TemplateContext::new(create_test_site_context())
        .with_section(create_test_section_context());

    // Test section template
    let result = renderer.render("section.html", &ctx);
    assert!(result.is_ok());

    let html = result.unwrap();
    assert!(html.contains("Blog"));
    assert!(html.contains("First Post"));
    assert!(html.contains("Second Post"));
    // Check that links are rendered (the exact format may vary due to escaping)
    assert!(html.contains("<a href="));
}

#[test]
fn test_render_with_draft_page() {
    let mut renderer = TeraRenderer::new().unwrap();

    renderer
        .register_template(
            "draft.html",
            r#"{% if page.draft %}<span class="draft">Draft</span>{% endif %}{{ page.title }}"#,
        )
        .unwrap();

    // Test non-draft page
    let ctx =
        TemplateContext::new(create_test_site_context()).with_page(create_test_page_context());
    let result = renderer.render("draft.html", &ctx).unwrap();
    assert!(!result.contains("Draft"));
    assert!(result.contains("Test Page"));

    // Test draft page
    let mut draft_page = create_test_page_context();
    draft_page.draft = true;
    let ctx = TemplateContext::new(create_test_site_context()).with_page(draft_page);
    let result = renderer.render("draft.html", &ctx).unwrap();
    assert!(result.contains("Draft"));
}

#[test]
fn test_render_with_date() {
    let mut renderer = TeraRenderer::new().unwrap();

    renderer
        .register_template(
            "date.html",
            r#"<time datetime="{{ page.date }}">{{ page.date }}</time>"#,
        )
        .unwrap();

    let ctx =
        TemplateContext::new(create_test_site_context()).with_page(create_test_page_context());
    let result = renderer.render("date.html", &ctx).unwrap();

    assert!(result.contains("2024-01-15"));
}

#[test]
fn test_render_with_load_templates_method() {
    let mut renderer = TeraRenderer::new().unwrap();

    // Initially no templates
    assert!(!renderer.has_template("base.html"));

    // Load templates
    let result =
        renderer.load_templates(PathBuf::from("tests/fixtures/template_site/templates").as_path());
    assert!(result.is_ok());

    // Now templates should be available
    assert!(renderer.has_template("base.html"));
    assert!(renderer.has_template("page.html"));
    assert!(renderer.has_template("section.html"));
}

#[test]
fn test_render_with_optional_fields() {
    let mut renderer = TeraRenderer::new().unwrap();

    renderer
        .register_template(
            "optional.html",
            r#"<article>
{% if page.description %}<p class="desc">{{ page.description }}</p>{% endif %}
{% if page.date %}<time>{{ page.date }}</time>{% endif %}
<h1>{{ page.title }}</h1>
</article>"#,
        )
        .unwrap();

    // Page with all optional fields
    let ctx =
        TemplateContext::new(create_test_site_context()).with_page(create_test_page_context());
    let html = renderer.render("optional.html", &ctx).unwrap();
    assert!(html.contains("A test page"));
    assert!(html.contains("2024-01-15"));

    // Page without optional fields
    let minimal_page = PageContext {
        title: "Minimal".to_string(),
        description: None,
        path: "/minimal/".to_string(),
        permalink: "https://test.example.com/minimal/".to_string(),
        content: String::new(),
        raw_content: String::new(),
        date: None,
        draft: false,
        summary: String::new(),
        word_count: 0,
        reading_time: 0,
        tags: vec![],
        categories: vec![],
        series: None,
    };
    let ctx = TemplateContext::new(create_test_site_context()).with_page(minimal_page);
    let html = renderer.render("optional.html", &ctx).unwrap();
    assert!(html.contains("Minimal"));
    assert!(!html.contains("class=\"desc\""));
    assert!(!html.contains("<time>"));
}

#[test]
fn test_render_site_context_only() {
    let mut renderer = TeraRenderer::new().unwrap();

    renderer
        .register_template(
            "site_only.html",
            r#"<footer>
<p>Site: {{ site.name }}</p>
<p>URL: {{ site.base_url }}</p>
{% if site.author %}<p>Author: {{ site.author }}</p>{% endif %}
{% if site.description %}<p>{{ site.description }}</p>{% endif %}
</footer>"#,
        )
        .unwrap();

    let ctx = TemplateContext::new(create_test_site_context());
    let html = renderer.render("site_only.html", &ctx).unwrap();

    assert!(html.contains("Integration Test Site"));
    // Check for base_url content (may be URL-encoded)
    assert!(html.contains("test.example.com") || html.contains("https://test.example.com"));
    assert!(html.contains("Test Author"));
    assert!(html.contains("Integration test site"));
}
