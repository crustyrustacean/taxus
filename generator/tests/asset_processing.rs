//! Integration tests for asset processing.

use generator::assets::{AssetProcessor, AssetReport, ScssProcessor, StaticCopier};
use generator::error::AssetError;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// =============================================================================
// SCSS Processor Tests
// =============================================================================

#[test]
fn test_scss_processor_basic() {
    let processor = ScssProcessor::with_include_paths(vec![PathBuf::from(
        "tests/fixtures/asset_site/styles",
    )]);
    let src = PathBuf::from("tests/fixtures/asset_site/styles/main.scss");
    let temp_dir = TempDir::new().unwrap();
    let dest = temp_dir.path().join("main.css");

    let result = processor.process(&src, &dest);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 1);
    assert_eq!(report.files_skipped, 0);
    assert!(!report.has_errors());

    // Check output file exists with .css extension
    let output = temp_dir.path().join("main.css");
    assert!(output.exists());

    // Check content is valid CSS
    let css = fs::read_to_string(&output).unwrap();
    assert!(css.contains("font-family"));
    assert!(css.contains("color"));
}

#[test]
fn test_scss_processor_with_imports() {
    let processor = ScssProcessor::with_include_paths(vec![PathBuf::from(
        "tests/fixtures/asset_site/styles",
    )]);
    let src = PathBuf::from("tests/fixtures/asset_site/styles/main.scss");
    let temp_dir = TempDir::new().unwrap();
    let dest = temp_dir.path().join("main.css");

    let result = processor.process(&src, &dest);

    assert!(result.is_ok());

    // Check that variables from _variables.scss are resolved
    let output = temp_dir.path().join("main.css");
    let css = fs::read_to_string(&output).unwrap();
    // Variables should be replaced with their values
    assert!(css.contains("-apple-system") || css.contains("BlinkMacSystemFont"));
}

#[test]
fn test_scss_processor_output_path() {
    let processor = ScssProcessor::with_include_paths(vec![PathBuf::from(
        "tests/fixtures/asset_site/styles",
    )]);
    let src = PathBuf::from("tests/fixtures/asset_site/styles/main.scss");
    let temp_dir = TempDir::new().unwrap();
    // Pass a path with .scss extension - should be changed to .css
    let dest = temp_dir.path().join("output/styles/main.scss");

    let result = processor.process(&src, &dest);

    assert!(result.is_ok());

    // Output should have .css extension
    let output = temp_dir.path().join("output/styles/main.css");
    assert!(output.exists());
}

#[test]
fn test_scss_processor_minified() {
    let processor = ScssProcessor::with_include_paths(vec![PathBuf::from(
        "tests/fixtures/asset_site/styles",
    )])
    .with_minify(true);
    let src = PathBuf::from("tests/fixtures/asset_site/styles/main.scss");
    let temp_dir = TempDir::new().unwrap();
    let dest = temp_dir.path().join("main.css");

    let result = processor.process(&src, &dest);

    assert!(result.is_ok());

    let output = temp_dir.path().join("main.css");
    let css = fs::read_to_string(&output).unwrap();

    // Minified CSS should be more compact (fewer newlines/whitespace)
    // Just verify it's valid CSS with expected content
    assert!(css.contains("font-family") || css.contains("body"));
}

#[test]
fn test_scss_processor_not_found() {
    let processor = ScssProcessor::new();
    let src = PathBuf::from("tests/fixtures/asset_site/styles/nonexistent.scss");
    let temp_dir = TempDir::new().unwrap();
    let dest = temp_dir.path().join("output.css");

    let result = processor.process(&src, &dest);

    assert!(result.is_err());
    match result.unwrap_err() {
        AssetError::NotFound(path) => {
            assert!(path.ends_with("nonexistent.scss"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

// =============================================================================
// Static Copier Tests
// =============================================================================

#[test]
fn test_static_copier_basic() {
    let copier = StaticCopier::new();
    let src = PathBuf::from("tests/fixtures/asset_site/static/scripts.js");
    let temp_dir = TempDir::new().unwrap();
    let dest = temp_dir.path().join("scripts.js");

    let result = copier.process(&src, &dest);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 1);
    assert_eq!(report.files_skipped, 0);
    assert!(!report.has_errors());

    // Check file was copied
    assert!(dest.exists());

    // Check content
    let content = fs::read_to_string(&dest).unwrap();
    assert!(content.contains("Asset processing test script"));
}

#[test]
fn test_static_copier_preserves_structure() {
    let copier = StaticCopier::new();
    let src = PathBuf::from("tests/fixtures/asset_site/static");
    let temp_dir = TempDir::new().unwrap();
    let dest = temp_dir.path().join("static");

    let result = copier.process(&src, &dest);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.files_processed >= 2); // At least scripts.js and config.json

    // Check directory structure is preserved
    assert!(dest.join("scripts.js").exists());
    assert!(dest.join("data/config.json").exists());

    // Check content
    let js_content = fs::read_to_string(dest.join("scripts.js")).unwrap();
    assert!(js_content.contains("greet"));

    let json_content = fs::read_to_string(dest.join("data/config.json")).unwrap();
    assert!(json_content.contains("Test Site"));
}

#[test]
fn test_static_copier_exclusions() {
    let copier = StaticCopier::with_exclusions(vec!["*.scss".to_string()]);
    let temp_dir = TempDir::new().unwrap();

    // Create test files
    let src_dir = temp_dir.path().join("source");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("style.scss"), "$color: blue;").unwrap();
    fs::write(src_dir.join("script.js"), "console.log('hi');").unwrap();

    let dest_dir = temp_dir.path().join("dest");
    let result = copier.process(&src_dir, &dest_dir);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.files_processed, 1); // Only script.js
    assert_eq!(report.files_skipped, 1); // style.scss excluded

    assert!(dest_dir.join("script.js").exists());
    assert!(!dest_dir.join("style.scss").exists());
}

#[test]
fn test_static_copier_not_found() {
    let copier = StaticCopier::new();
    let src = PathBuf::from("tests/fixtures/asset_site/static/nonexistent");
    let temp_dir = TempDir::new().unwrap();
    let dest = temp_dir.path().join("output");

    let result = copier.process(&src, &dest);

    assert!(result.is_err());
    match result.unwrap_err() {
        AssetError::NotFound(path) => {
            assert!(path.ends_with("nonexistent"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

// =============================================================================
// Combined Asset Processing Tests
// =============================================================================

#[test]
fn test_combined_asset_processing() {
    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    // Process SCSS
    let scss_processor = ScssProcessor::with_include_paths(vec![PathBuf::from(
        "tests/fixtures/asset_site/styles",
    )]);
    let scss_src = PathBuf::from("tests/fixtures/asset_site/styles/main.scss");
    let scss_dest = output_dir.join("styles/main.css");
    let scss_report = scss_processor.process(&scss_src, &scss_dest).unwrap();
    assert_eq!(scss_report.files_processed, 1);

    // Process static files
    let static_copier = StaticCopier::new();
    let static_src = PathBuf::from("tests/fixtures/asset_site/static");
    let static_dest = output_dir.join("static");
    let static_report = static_copier.process(&static_src, &static_dest).unwrap();
    assert!(static_report.files_processed >= 2);

    // Verify all outputs exist
    assert!(output_dir.join("styles/main.css").exists());
    assert!(output_dir.join("static/scripts.js").exists());
    assert!(output_dir.join("static/data/config.json").exists());
}

#[test]
fn test_asset_report_aggregation() {
    let mut total_report = AssetReport::new();

    // Process multiple assets and merge reports
    let temp_dir = TempDir::new().unwrap();

    // Process SCSS
    let scss_processor = ScssProcessor::with_include_paths(vec![PathBuf::from(
        "tests/fixtures/asset_site/styles",
    )]);
    let scss_src = PathBuf::from("tests/fixtures/asset_site/styles/main.scss");
    let scss_dest = temp_dir.path().join("main.css");
    let scss_report = scss_processor.process(&scss_src, &scss_dest).unwrap();
    total_report.merge(scss_report);

    // Process static
    let static_copier = StaticCopier::new();
    let static_src = PathBuf::from("tests/fixtures/asset_site/static");
    let static_dest = temp_dir.path().join("static");
    let static_report = static_copier.process(&static_src, &static_dest).unwrap();
    total_report.merge(static_report);

    // Check totals
    assert!(total_report.files_processed >= 3); // 1 SCSS + at least 2 static
    assert!(!total_report.has_errors());
}

// =============================================================================
// AssetProcessor Trait Tests
// =============================================================================

#[test]
fn test_scss_processor_handles_trait() {
    let processor = ScssProcessor::new();

    assert!(processor.handles(Path::new("styles/main.scss")));
    assert!(processor.handles(Path::new("theme.sass")));
    assert!(!processor.handles(Path::new("styles/main.css")));
    assert!(!processor.handles(Path::new("script.js")));
}

#[test]
fn test_static_copier_handles_trait() {
    let copier = StaticCopier::new();

    // StaticCopier handles all files
    assert!(copier.handles(Path::new("image.png")));
    assert!(copier.handles(Path::new("script.js")));
    assert!(copier.handles(Path::new("style.css")));
    assert!(copier.handles(Path::new("data.json")));
}

#[test]
fn test_processor_names() {
    let scss_processor = ScssProcessor::new();
    let static_copier = StaticCopier::new();

    assert_eq!(scss_processor.name(), "scss");
    assert_eq!(static_copier.name(), "static");
}
