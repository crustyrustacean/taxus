//! End-to-end tests for `--dry-run` builds.
//!
//! A dry-run build must complete all processing stages (parsing, rendering,
//! SCSS compilation) without creating the output directory at all.

use std::path::Path;
use taxus_lib::build::SiteBuilder;
use taxus_lib::config::SiteConfig;
use tempfile::TempDir;

#[test]
fn test_build_dry_run_writes_nothing() {
    let fixture_dir = Path::new("tests/fixtures/internal_links_site");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join("dist");

    // Load config and redirect output to a location that must stay empty
    let mut config = SiteConfig::from_dir(fixture_dir).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();

    // Dry-run build must succeed
    let report = SiteBuilder::new(config)
        .dry_run(true)
        .build()
        .expect("Dry-run build failed");

    // The build still processes content
    assert!(report.pages_rendered + report.sections_rendered > 0);

    // But the output directory must not exist at all: no HTML, no CSS,
    // no static files, no WASM client
    assert!(
        !output_dir.exists(),
        "dry-run build created the output directory: {}",
        output_dir.display()
    );
}

#[test]
fn test_build_dry_run_scss_errors_still_surface() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let site_dir = temp_dir.path().join("site");

    // Scaffold a minimal site with invalid SCSS
    std::fs::create_dir_all(site_dir.join("content")).unwrap();
    std::fs::create_dir_all(site_dir.join("templates")).unwrap();
    std::fs::create_dir_all(site_dir.join("styles")).unwrap();
    std::fs::write(
        site_dir.join("site.toml"),
        "[site]\nname = \"T\"\nbase_url = \"https://example.com\"\n",
    )
    .unwrap();
    std::fs::write(
        site_dir.join("content/_index.md"),
        "+++\ntitle = \"Home\"\n+++\nHello",
    )
    .unwrap();
    std::fs::write(
        site_dir.join("templates/page.html"),
        "<html><body>{{ page.content }}</body></html>",
    )
    .unwrap();
    std::fs::write(
        site_dir.join("templates/section.html"),
        "<html><body>{{ page.content }}</body></html>",
    )
    .unwrap();
    std::fs::write(
        site_dir.join("templates/base.html"),
        "<html><body>{% block content %}{% endblock %}</body></html>",
    )
    .unwrap();
    std::fs::write(site_dir.join("styles/main.scss"), "body { color: }").unwrap();

    let output_dir = temp_dir.path().join("dist");
    let mut config = SiteConfig::from_dir(&site_dir).expect("Failed to load config");
    config.build.output_dir = output_dir.clone();

    // SCSS is still compiled in dry-run: invalid syntax surfaces as an asset
    // error in the build report (directory-mode SCSS errors are reported as
    // warnings, not build failures)
    let report = SiteBuilder::new(config)
        .dry_run(true)
        .build()
        .expect("dry-run build should complete");

    assert!(
        report.assets.has_errors(),
        "invalid SCSS should surface as an asset error"
    );
    assert!(report.sections_rendered + report.pages_rendered > 0);

    // And still nothing was written
    assert!(!output_dir.exists());
}
