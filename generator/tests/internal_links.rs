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
