//! Integration tests for internal link resolution.

use std::path::Path;
use tempfile::TempDir;
use yew_ssg_lib::build::SiteBuilder;
use yew_ssg_lib::config::SiteConfig;

/// Test that internal links are correctly resolved in built output.
#[test]
fn test_internal_links_resolved_in_output() {
    let fixture_dir = Path::new("tests/fixtures/internal_links_site");

    // Create a temporary output directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().to_path_buf();

    // Load config and override output directory
    let mut config = SiteConfig::from_dir(fixture_dir).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();

    // Build the site
    let builder = SiteBuilder::new(config);
    let result = builder.build();

    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    // Check the home page output
    let home_output = std::fs::read_to_string(output_dir.join("index.html"))
        .expect("Failed to read home page output");

    // The internal link @/about.md should be resolved to /about/
    assert!(
        home_output.contains("href=\"/about/\""),
        "Home page should contain resolved link to /about/. Content: {}",
        home_output
    );

    // The internal link @/blog/first-post.md should be resolved to /blog/first-post/
    assert!(
        home_output.contains("href=\"/blog/first-post/\""),
        "Home page should contain resolved link to /blog/first-post/. Content: {}",
        home_output
    );

    // Check the about page output
    let about_output = std::fs::read_to_string(output_dir.join("about/index.html"))
        .expect("Failed to read about page output");

    // The internal link @/_index.md should be resolved to /
    assert!(
        about_output.contains("href=\"/\""),
        "About page should contain resolved link to /. Content: {}",
        about_output
    );

    // Check the blog post output
    let blog_post_output = std::fs::read_to_string(output_dir.join("blog/first-post/index.html"))
        .expect("Failed to read blog post output");

    // The internal link @/about.md should be resolved to /about/
    assert!(
        blog_post_output.contains("href=\"/about/\""),
        "Blog post should contain resolved link to /about/. Content: {}",
        blog_post_output
    );
}

/// Test that word_count and reading_time are available in section pages.
#[test]
fn test_section_pages_have_word_count_and_reading_time() {
    let fixture_dir = Path::new("tests/fixtures/internal_links_site");

    // Create a temporary output directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().to_path_buf();

    // Load config and override output directory
    let mut config = SiteConfig::from_dir(fixture_dir).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();

    // Build the site
    let builder = SiteBuilder::new(config);
    let result = builder.build();

    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    // Check the blog section output
    let blog_output = std::fs::read_to_string(output_dir.join("blog/index.html"))
        .expect("Failed to read blog section output");

    // The blog section should list child pages
    assert!(
        blog_output.contains("First Post"),
        "Blog section should contain 'First Post'. Content: {}",
        blog_output
    );
}

/// Test that word_count and reading_time are correctly computed and available in templates.
#[test]
fn test_word_count_and_reading_time_in_section() {
    use yew_ssg_lib::content::Page;

    // Load the first-post.md page
    let page = Page::from_file("tests/fixtures/internal_links_site/content/blog/first-post.md")
        .expect("Failed to load page");

    // The content is: "# First Post\n\nThis is my first blog post. Check out the [about page](@/about.md) for more info.\n"
    // Words should be counted correctly
    assert!(page.word_count() > 0, "Word count should be greater than 0");
    assert!(page.reading_time() > 0, "Reading time should be greater than 0");

    // Print for debugging
    println!("Word count: {}", page.word_count());
    println!("Reading time: {}", page.reading_time());
    println!("Raw content: {:?}", page.raw_content);
}

/// Test that word_count and reading_time are rendered in section output when template uses them.
#[test]
fn test_word_count_reading_time_rendered_in_section() {
    use std::io::Write;

    let fixture_dir = Path::new("tests/fixtures/internal_links_site");

    // Create a temporary output directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().to_path_buf();

    // Create a temporary templates directory with a modified section template
    let templates_dir = temp_dir.path().join("templates");
    std::fs::create_dir_all(&templates_dir).expect("Failed to create templates dir");

    // Copy base.html
    std::fs::copy(
        fixture_dir.join("templates/base.html"),
        templates_dir.join("base.html"),
    )
    .expect("Failed to copy base.html");

    // Copy page.html
    std::fs::copy(
        fixture_dir.join("templates/page.html"),
        templates_dir.join("page.html"),
    )
    .expect("Failed to copy page.html");

    // Create a modified section.html that includes word_count and reading_time
    let section_template = r#"{% extends "base.html" %} {% block title %}{{ section.title }} - {{ site.name }}{% endblock %} {% block content %}
<section>
  <h1>{{ section.title }}</h1>
  {{ page.content | safe }}
  <ul>
    {% for page in section.pages %}
    <li>
      <a href="{{ page.path }}">{{ page.title }}</a>
      <span class="word-count">{{ page.word_count }} words</span>
      <span class="reading-time">{{ page.reading_time }} min read</span>
    </li>
    {% endfor %}
  </ul>
</section>
{% endblock %}"#;

    let mut file = std::fs::File::create(templates_dir.join("section.html")).expect("Failed to create section.html");
    file.write_all(section_template.as_bytes()).expect("Failed to write section.html");

    // Copy content directory
    fn copy_dir_all(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).expect("Failed to create dir");
        for entry in std::fs::read_dir(src).expect("Failed to read dir") {
            let entry = entry.expect("Failed to get entry");
            let ty = entry.file_type().expect("Failed to get file type");
            if ty.is_dir() {
                copy_dir_all(&entry.path(), &dst.join(entry.file_name()));
            } else {
                std::fs::copy(entry.path(), dst.join(entry.file_name())).expect("Failed to copy file");
            }
        }
    }

    let content_dir = temp_dir.path().join("content");
    copy_dir_all(&fixture_dir.join("content"), &content_dir);

    // Create site.toml
    std::fs::copy(fixture_dir.join("site.toml"), temp_dir.path().join("site.toml"))
        .expect("Failed to copy site.toml");

    // Load config from temp directory
    let mut config = SiteConfig::from_dir(temp_dir.path()).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();

    // Build the site
    let builder = SiteBuilder::new(config);
    let result = builder.build();

    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    // Check the blog section output
    let blog_output = std::fs::read_to_string(output_dir.join("blog/index.html"))
        .expect("Failed to read blog section output");

    // The blog section should contain word count and reading time
    assert!(
        blog_output.contains("words"),
        "Blog section should contain 'words'. Content: {}",
        blog_output
    );
    assert!(
        blog_output.contains("min read"),
        "Blog section should contain 'min read'. Content: {}",
        blog_output
    );
}
