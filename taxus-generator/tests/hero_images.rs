//! Integration tests for hero image processing.

use std::fs;
use taxus_lib::build::SiteBuilder;
use taxus_lib::config::{
    BuildConfig, FeedConfig, HighlightConfig, ImageConfig, MarkdownConfig, SiteConfig, SiteMeta,
};
use taxus_lib::images::{ImageProcessor, ImageRegistry, render_picture};
use tempfile::TempDir;

fn create_hero_site(temp: &TempDir) -> SiteConfig {
    let content_dir = temp.path().join("content");
    let blog_dir = content_dir.join("blog");
    let templates_dir = temp.path().join("templates");
    let styles_dir = temp.path().join("styles");
    let static_dir = temp.path().join("static");

    fs::create_dir_all(&blog_dir).unwrap();
    fs::create_dir_all(&templates_dir).unwrap();
    fs::create_dir_all(&styles_dir).unwrap();
    fs::create_dir_all(&static_dir).unwrap();

    fs::write(
        content_dir.join("_index.md"),
        r#"+++
title = "Home"
description = "Welcome"
+++

# Welcome
"#,
    )
    .unwrap();

    fs::write(
        blog_dir.join("_index.md"),
        r#"+++
title = "Blog"
+++

Blog section.
"#,
    )
    .unwrap();

    fs::write(
        blog_dir.join("hero-post.md"),
        r#"+++
title = "Hero Post"
description = "A post with a hero image"
hero_image = "hero.jpg"
hero_alt = "A beautiful sunset"
date = 2024-03-15
+++

# Hero Post

This post has a hero image.
"#,
    )
    .unwrap();

    let img = image::RgbImage::from_pixel(1600, 900, image::Rgb([100, 150, 200]));
    img.save(blog_dir.join("hero.jpg")).unwrap();

    fs::write(
        templates_dir.join("base.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>{% block title %}{{ site.name }}{% endblock %}</title>
</head>
<body>
    <main>{% block content %}{% endblock %}</main>
</body>
</html>"#,
    )
    .unwrap();

    fs::write(
        templates_dir.join("page.html"),
        r#"{% extends "base.html" %}
{% block title %}{{ page.title }} - {{ site.name }}{% endblock %}
{% block content %}
<article>
    {% if page.hero %}
    <picture>
      <source srcset="{{ page.hero.srcset | safe }}" type="{{ page.hero.mime_type | safe }}">
      <img src="{{ page.hero.src | safe }}" alt="{{ page.hero.alt }}"
           width="{{ page.hero.width }}" height="{{ page.hero.height }}"
           loading="eager" decoding="async">
    </picture>
    {% endif %}
    <h1>{{ page.title }}</h1>
    {{ page.content | safe }}
</article>
{% endblock %}"#,
    )
    .unwrap();

    fs::write(
        templates_dir.join("section.html"),
        r#"{% extends "base.html" %}
{% block title %}{{ page.title }} - {{ site.name }}{% endblock %}
{% block content %}
<section>
    <h1>{{ page.title }}</h1>
    {% if section.pages %}
    <ul>
    {% for p in section.pages %}
    <li>{{ p.title }}</li>
    {% endfor %}
    </ul>
    {% endif %}
</section>
{% endblock %}"#,
    )
    .unwrap();

    let output_dir = temp.path().join("dist");

    SiteConfig {
        site: SiteMeta {
            name: "Hero Test Site".to_string(),
            base_url: "https://hero.example.com".to_string(),
            description: None,
            author: None,
        },
        build: BuildConfig {
            content_dir,
            output_dir,
            static_dir,
            styles_dir,
            templates_dir,
            slugify: "on".to_string(),
        },
        feed: FeedConfig::default(),
        highlight: HighlightConfig::default(),
        images: ImageConfig::default(),
        markdown: MarkdownConfig::default(),
        base_dir: temp.path().to_path_buf(),
    }
}

#[test]
fn test_hero_image_build_produces_variants() {
    let temp = TempDir::new().unwrap();
    let config = create_hero_site(&temp);

    let report = SiteBuilder::new(config.clone()).build().unwrap();

    assert!(report.pages_rendered > 0);

    let images_dir = config.build.output_dir.join("images");
    assert!(images_dir.exists(), "Images directory should exist in dist");

    let entries: Vec<_> = fs::read_dir(&images_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    let webp_files: Vec<_> = entries
        .iter()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "webp"))
        .collect();

    assert!(
        webp_files.len() >= 3,
        "Should have at least 3 WebP variants, found {}",
        webp_files.len()
    );

    for file in &webp_files {
        let name = file.file_name().to_string_lossy().to_string();
        assert!(
            name.contains("hero-"),
            "Variant filename should start with 'hero-', got: {}",
            name
        );
        assert!(
            name.contains("w.webp"),
            "Variant filename should contain width descriptor, got: {}",
            name
        );
    }
}

#[test]
fn test_hero_image_html_contains_picture() {
    let temp = TempDir::new().unwrap();
    let config = create_hero_site(&temp);

    SiteBuilder::new(config.clone()).build().unwrap();

    let hero_html_path = config.build.output_dir.join("blog/hero-post/index.html");
    assert!(hero_html_path.exists(), "Hero post HTML should exist");

    let html = fs::read_to_string(&hero_html_path).unwrap();

    assert!(
        html.contains("<picture>"),
        "HTML should contain <picture> element"
    );
    assert!(
        html.contains("</picture>"),
        "HTML should contain closing </picture>"
    );
    assert!(
        html.contains("<source"),
        "HTML should contain <source> element"
    );
    assert!(
        html.contains("type=\"image/webp\""),
        "HTML should specify image/webp type"
    );
    assert!(
        html.contains("srcset="),
        "HTML should contain srcset attribute"
    );
    assert!(html.contains("400w"), "HTML should reference 400w variant");
    assert!(html.contains("800w"), "HTML should reference 800w variant");
    assert!(
        html.contains("1200w"),
        "HTML should reference 1200w variant"
    );
    assert!(
        html.contains("alt=\"A beautiful sunset\""),
        "HTML should use hero_alt text"
    );
    assert!(
        html.contains("width=\"1600\""),
        "HTML should have original width"
    );
    assert!(
        html.contains("height=\"900\""),
        "HTML should have original height"
    );
    assert!(
        html.contains("loading=\"eager\""),
        "Hero should use loading=eager"
    );
    assert!(
        html.contains("decoding=\"async\""),
        "HTML should have decoding=async"
    );
}

#[test]
fn test_home_page_without_hero_no_picture() {
    let temp = TempDir::new().unwrap();
    let config = create_hero_site(&temp);

    SiteBuilder::new(config.clone()).build().unwrap();

    let home_html_path = config.build.output_dir.join("index.html");
    assert!(home_html_path.exists(), "Home page HTML should exist");

    let html = fs::read_to_string(&home_html_path).unwrap();
    assert!(
        !html.contains("<picture>"),
        "Home page without hero should not have <picture>"
    );
}

#[test]
fn test_hero_image_alt_fallback_to_title() {
    let temp = TempDir::new().unwrap();
    let content_dir = temp.path().join("content");
    let templates_dir = temp.path().join("templates");
    let styles_dir = temp.path().join("styles");
    let static_dir = temp.path().join("static");
    let output_dir = temp.path().join("dist");

    fs::create_dir_all(&content_dir).unwrap();
    fs::create_dir_all(&templates_dir).unwrap();
    fs::create_dir_all(&styles_dir).unwrap();
    fs::create_dir_all(&static_dir).unwrap();

    fs::write(
        content_dir.join("no-alt.md"),
        r#"+++
title = "No Alt Post"
hero_image = "photo.jpg"
+++

Content without hero_alt.
"#,
    )
    .unwrap();

    let img = image::RgbImage::from_pixel(800, 600, image::Rgb([200, 100, 50]));
    img.save(content_dir.join("photo.jpg")).unwrap();

    fs::write(
        templates_dir.join("base.html"),
        r#"<!DOCTYPE html><html><head><title>{{ site.name }}</title></head><body>{% block content %}{% endblock %}</body></html>"#,
    )
    .unwrap();

    fs::write(
        templates_dir.join("page.html"),
        r#"{% extends "base.html" %}{% block content %}<article>{% if page.hero %}<picture><source srcset="{{ page.hero.srcset | safe }}" type="{{ page.hero.mime_type | safe }}"><img src="{{ page.hero.src | safe }}" alt="{{ page.hero.alt }}" width="{{ page.hero.width }}" height="{{ page.hero.height }}" loading="eager" decoding="async"></picture>{% endif %}<h1>{{ page.title }}</h1>{{ page.content | safe }}</article>{% endblock %}"#,
    )
    .unwrap();

    let config = SiteConfig {
        site: SiteMeta {
            name: "Alt Fallback Test".to_string(),
            base_url: "https://example.com".to_string(),
            description: None,
            author: None,
        },
        build: BuildConfig {
            content_dir,
            output_dir,
            static_dir,
            styles_dir,
            templates_dir,
            slugify: "on".to_string(),
        },
        feed: FeedConfig::default(),
        highlight: HighlightConfig::default(),
        images: ImageConfig::default(),
        markdown: MarkdownConfig::default(),
        base_dir: temp.path().to_path_buf(),
    };

    SiteBuilder::new(config.clone()).build().unwrap();

    let html_path = config.build.output_dir.join("no-alt/index.html");
    let html = fs::read_to_string(&html_path).unwrap();

    assert!(
        html.contains("alt=\"No Alt Post\""),
        "When hero_alt is not set, should fall back to page title, got: {}",
        html
    );
}

#[test]
fn test_image_processor_creates_webp_variants() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let output_dir = temp.path().join("dist");
    fs::create_dir_all(&source_dir).unwrap();

    let img = image::RgbImage::from_pixel(1600, 900, image::Rgb([128, 128, 128]));
    let source_path = source_dir.join("test.jpg");
    img.save(&source_path).unwrap();

    let processor = ImageProcessor::new(ImageConfig::default(), output_dir.clone());
    let result = processor.process(&source_path, "Test image").unwrap();

    assert_eq!(result.meta.variants.len(), 3);
    assert_eq!(result.meta.original_width, 1600);
    assert_eq!(result.meta.original_height, 900);
    assert_eq!(result.mime_type(), "image/webp");

    for variant in &result.meta.variants {
        assert!(
            variant.path.exists(),
            "Variant file should exist: {:?}",
            variant.path
        );
        assert!(
            variant.file_size > 0,
            "Variant should have non-zero file size"
        );
    }
}

#[test]
fn test_picture_html_generation() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let output_dir = temp.path().join("dist");
    fs::create_dir_all(&source_dir).unwrap();

    let img = image::RgbImage::from_pixel(1600, 900, image::Rgb([128, 128, 128]));
    let source_path = source_dir.join("test.jpg");
    img.save(&source_path).unwrap();

    let processor = ImageProcessor::new(ImageConfig::default(), output_dir);
    let processed = processor.process(&source_path, "Test image").unwrap();

    let html = render_picture(&processed, "Test image", "eager");

    assert!(html.contains("<picture>"));
    assert!(html.contains("<source"));
    assert!(html.contains("type=\"image/webp\""));
    assert!(html.contains("<img"));
    assert!(html.contains("alt=\"Test image\""));
    assert!(html.contains("width=\"1600\""));
    assert!(html.contains("height=\"900\""));
    assert!(html.contains("loading=\"eager\""));
    assert!(html.contains("decoding=\"async\""));
    assert!(html.contains("400w"));
    assert!(html.contains("800w"));
    assert!(html.contains("1200w"));
}

#[test]
fn test_image_registry() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let output_dir = temp.path().join("dist");
    fs::create_dir_all(&source_dir).unwrap();

    let img = image::RgbImage::from_pixel(800, 600, image::Rgb([128, 128, 128]));
    let source_path = source_dir.join("test.jpg");
    img.save(&source_path).unwrap();

    let processor = ImageProcessor::new(ImageConfig::default(), output_dir);
    let processed = processor.process(&source_path, "Test").unwrap();

    let mut registry = ImageRegistry::new();
    let key = source_path.clone();
    registry.insert(key.clone(), processed);

    assert_eq!(registry.len(), 1);
    let retrieved = registry.get(&key).unwrap();
    assert_eq!(retrieved.meta.original_width, 800);
    assert_eq!(retrieved.meta.original_height, 600);
}
