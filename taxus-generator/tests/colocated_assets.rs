//! Integration tests for co-located asset copying.
//!
//! Co-located assets are non-markdown files in the content directory that should
//! be copied as-is to the output directory, preserving their relative paths.

use std::fs;
use std::path::PathBuf;
use taxus_lib::assets::AssetReport;
use taxus_lib::build::pipeline::copy_colocated_assets;
use tempfile::TempDir;

// =============================================================================
// Basic Co-located Asset Tests
// =============================================================================

#[test]
fn test_copy_colocated_assets_basic() {
    let content_dir = PathBuf::from("tests/fixtures/colocated_site/content");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, false);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 2); // photo.jpg and headshot.png
    assert_eq!(report.files_skipped, 0);
    assert!(!report.has_errors());

    // Check files were copied to correct paths
    assert!(output_dir.join("blog/photo.jpg").exists());
    assert!(output_dir.join("about/headshot.png").exists());
}

#[test]
fn test_copy_colocated_assets_skips_markdown() {
    let content_dir = PathBuf::from("tests/fixtures/colocated_site/content");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, false);

    assert!(result.is_ok());

    // Markdown files should NOT be copied
    assert!(!output_dir.join("_index.md").exists());
    assert!(!output_dir.join("blog/_index.md").exists());
    assert!(!output_dir.join("blog/first-post.md").exists());
    assert!(!output_dir.join("about/about.md").exists());
}

#[test]
fn test_copy_colocated_assets_preserves_content() {
    let content_dir = PathBuf::from("tests/fixtures/colocated_site/content");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, false);
    assert!(result.is_ok());

    // Verify content is preserved (binary files should be identical)
    let original_photo = fs::read("tests/fixtures/colocated_site/content/blog/photo.jpg").unwrap();
    let copied_photo = fs::read(output_dir.join("blog/photo.jpg")).unwrap();
    assert_eq!(original_photo, copied_photo);

    let original_headshot =
        fs::read("tests/fixtures/colocated_site/content/about/headshot.png").unwrap();
    let copied_headshot = fs::read(output_dir.join("about/headshot.png")).unwrap();
    assert_eq!(original_headshot, copied_headshot);
}

#[test]
fn test_copy_colocated_assets_creates_directories() {
    let content_dir = PathBuf::from("tests/fixtures/colocated_site/content");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    // Output directory doesn't exist yet
    assert!(!output_dir.exists());

    let result = copy_colocated_assets(&content_dir, &output_dir, false);

    assert!(result.is_ok());

    // Directories should be created
    assert!(output_dir.exists());
    assert!(output_dir.join("blog").exists());
    assert!(output_dir.join("about").exists());
}

// =============================================================================
// Dry Run Tests
// =============================================================================

#[test]
fn test_copy_colocated_assets_dry_run() {
    let content_dir = PathBuf::from("tests/fixtures/colocated_site/content");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, true);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 2);

    // In dry run, files should NOT be written
    assert!(!output_dir.join("blog/photo.jpg").exists());
    assert!(!output_dir.join("about/headshot.png").exists());
}

// =============================================================================
// Empty Directory Tests
// =============================================================================

#[test]
fn test_copy_colocated_assets_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create empty content directory
    let content_dir = temp_dir.path().join("content");
    fs::create_dir_all(&content_dir).unwrap();

    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, false);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 0);
    assert_eq!(report.files_skipped, 0);
}

#[test]
fn test_copy_colocated_assets_only_markdown() {
    let temp_dir = TempDir::new().unwrap();

    // Create content directory with only markdown files
    let content_dir = temp_dir.path().join("content");
    fs::create_dir_all(content_dir.join("posts")).unwrap();
    fs::write(content_dir.join("_index.md"), "---\ntitle: Home\n---\n").unwrap();
    fs::write(
        content_dir.join("posts/first.md"),
        "---\ntitle: First\n---\n",
    )
    .unwrap();

    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, false);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 0); // No non-markdown files

    // No files should be copied
    assert!(!output_dir.join("_index.md").exists());
    assert!(!output_dir.join("posts/first.md").exists());
}

// =============================================================================
// Nested Directory Tests
// =============================================================================

#[test]
fn test_copy_colocated_assets_nested_directories() {
    let temp_dir = TempDir::new().unwrap();

    // Create content directory with nested structure
    let content_dir = temp_dir.path().join("content");
    fs::create_dir_all(content_dir.join("a/b/c")).unwrap();
    fs::write(content_dir.join("a/b/c/deep.txt"), "deep file").unwrap();
    fs::create_dir_all(content_dir.join("x/y")).unwrap();
    fs::write(content_dir.join("x/y/nested.json"), r#"{"key": "value"}"#).unwrap();

    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, false);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 2);

    // Check nested paths are preserved
    assert!(output_dir.join("a/b/c/deep.txt").exists());
    assert!(output_dir.join("x/y/nested.json").exists());

    // Verify content
    let deep_content = fs::read_to_string(output_dir.join("a/b/c/deep.txt")).unwrap();
    assert_eq!(deep_content, "deep file");
}

// =============================================================================
// Report Aggregation Tests
// =============================================================================

#[test]
fn test_colocated_assets_report_aggregation() {
    let content_dir = PathBuf::from("tests/fixtures/colocated_site/content");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    let colocated_report = copy_colocated_assets(&content_dir, &output_dir, false).unwrap();

    // Simulate merging with other asset reports
    let mut total_report = AssetReport::new();
    total_report.merge(colocated_report);

    assert_eq!(total_report.files_processed, 2);
    assert!(!total_report.has_errors());
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_copy_colocated_assets_nonexistent_source() {
    let content_dir = PathBuf::from("tests/fixtures/nonexistent_content");
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, false);

    // Should return Ok with empty report for non-existent source directory
    // (graceful handling - the directory might not exist yet)
    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 0);
    assert_eq!(report.files_skipped, 0);
}

// =============================================================================
// Various File Types Tests
// =============================================================================

#[test]
fn test_copy_colocated_assets_various_file_types() {
    let temp_dir = TempDir::new().unwrap();

    // Create content directory with various file types
    let content_dir = temp_dir.path().join("content");
    fs::create_dir_all(&content_dir).unwrap();
    fs::write(content_dir.join("data.json"), r#"{"test": true}"#).unwrap();
    fs::write(content_dir.join("style.css"), "body { color: red; }").unwrap();
    fs::write(content_dir.join("script.js"), "console.log('hi');").unwrap();
    fs::write(content_dir.join("image.svg"), "<svg></svg>").unwrap();
    fs::write(content_dir.join("doc.pdf"), "%PDF-1.4").unwrap();
    fs::write(content_dir.join("page.md"), "---\ntitle: Test\n---\n").unwrap();

    let output_dir = temp_dir.path().join("dist");

    let result = copy_colocated_assets(&content_dir, &output_dir, false);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 5); // All except .md file
    assert_eq!(report.files_skipped, 0);

    // Verify all non-markdown files were copied
    assert!(output_dir.join("data.json").exists());
    assert!(output_dir.join("style.css").exists());
    assert!(output_dir.join("script.js").exists());
    assert!(output_dir.join("image.svg").exists());
    assert!(output_dir.join("doc.pdf").exists());

    // Markdown should not be copied
    assert!(!output_dir.join("page.md").exists());
}
