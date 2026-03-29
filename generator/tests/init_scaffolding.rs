//! Integration tests for site initialization.

use std::path::PathBuf;
use tempfile::TempDir;
use yew_ssg_lib::TemplateRenderer;
use yew_ssg_lib::error::InitError;
use yew_ssg_lib::init::{InitOptions, InitScaffolder, derive_site_name, is_directory_empty};

#[test]
fn test_init_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let options = InitOptions::new("Test Site", "https://test.example.com");
    let scaffolder = InitScaffolder::new(options);

    let report = scaffolder.scaffold(temp_dir.path()).unwrap();

    assert_eq!(report.directories_created, 4);
    assert_eq!(report.files_created, 9);
    assert!(temp_dir.path().join("site.toml").exists());
}

#[test]
fn test_init_creates_valid_config() {
    let temp_dir = TempDir::new().unwrap();
    let options = InitOptions::new("My Site", "https://my-site.example.com");
    let scaffolder = InitScaffolder::new(options);

    scaffolder.scaffold(temp_dir.path()).unwrap();

    // Verify the config can be loaded
    let config = yew_ssg_lib::config::SiteConfig::from_dir(temp_dir.path()).unwrap();
    assert_eq!(config.site.name, "My Site");
    assert_eq!(config.site.base_url, "https://my-site.example.com");
}

#[test]
fn test_init_creates_valid_content() {
    let temp_dir = TempDir::new().unwrap();
    let options = InitOptions::new("Test Site", "https://test.example.com");
    let scaffolder = InitScaffolder::new(options);

    scaffolder.scaffold(temp_dir.path()).unwrap();

    // Verify the index page can be loaded
    let page =
        yew_ssg_lib::content::Page::from_file(temp_dir.path().join("content/_index.md")).unwrap();
    assert_eq!(page.frontmatter.title, "Home");
}

#[test]
fn test_init_creates_valid_templates() {
    let temp_dir = TempDir::new().unwrap();
    let options = InitOptions::new("Test Site", "https://test.example.com");
    let scaffolder = InitScaffolder::new(options);

    scaffolder.scaffold(temp_dir.path()).unwrap();

    // Verify templates can be loaded
    let renderer =
        yew_ssg_lib::templates::TeraRenderer::from_dir(temp_dir.path().join("templates")).unwrap();
    assert!(renderer.has_template("base.html"));
    assert!(renderer.has_template("page.html"));
    assert!(renderer.has_template("section.html"));
}

#[test]
fn test_init_with_custom_name_and_url() {
    let temp_dir = TempDir::new().unwrap();
    let options = InitOptions::new("Custom Name", "https://custom.example.org");
    let scaffolder = InitScaffolder::new(options);

    scaffolder.scaffold(temp_dir.path()).unwrap();

    let config = yew_ssg_lib::config::SiteConfig::from_dir(temp_dir.path()).unwrap();
    assert_eq!(config.site.name, "Custom Name");
    assert_eq!(config.site.base_url, "https://custom.example.org");
}

#[test]
fn test_init_does_not_overwrite_existing_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create an existing config
    let config_path = temp_dir.path().join("site.toml");
    std::fs::write(
        &config_path,
        "[site]\nname = \"Existing\"\nbase_url = \"https://existing.com\"",
    )
    .unwrap();

    let options = InitOptions::new("New Site", "https://new.example.com");
    let scaffolder = InitScaffolder::new(options);

    scaffolder.scaffold(temp_dir.path()).unwrap();

    // Verify the existing config was not overwritten
    let content = std::fs::read_to_string(config_path).unwrap();
    assert!(content.contains("Existing"));
    assert!(!content.contains("New Site"));
}

#[test]
fn test_init_validates_options() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_options = InitOptions::new("", "https://test.example.com");
    let scaffolder = InitScaffolder::new(invalid_options);

    let result = scaffolder.scaffold(temp_dir.path());
    assert!(result.is_err());
}

#[test]
fn test_is_directory_empty_with_empty_dir() {
    let temp_dir = TempDir::new().unwrap();
    assert!(is_directory_empty(temp_dir.path()).unwrap());
}

#[test]
fn test_is_directory_empty_with_files() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
    assert!(!is_directory_empty(temp_dir.path()).unwrap());
}

#[test]
fn test_is_directory_empty_with_nonexistent() {
    assert!(is_directory_empty(PathBuf::from("nonexistent_dir_12345").as_path()).unwrap());
}

#[test]
fn test_derive_site_name() {
    assert_eq!(
        derive_site_name(PathBuf::from("my-site").as_path()),
        "my-site"
    );
    assert_eq!(
        derive_site_name(PathBuf::from("/path/to/my-site").as_path()),
        "my-site"
    );
    assert_eq!(derive_site_name(PathBuf::from(".").as_path()), "My Site");
}

#[test]
fn test_init_options_validation() {
    // Valid options
    let valid = InitOptions::new("Test", "https://example.com");
    assert!(valid.validate().is_ok());

    // Empty name
    let empty_name = InitOptions::new("", "https://example.com");
    assert!(matches!(
        empty_name.validate(),
        Err(InitError::InvalidName(_))
    ));

    // Empty base URL
    let empty_url = InitOptions::new("Test", "");
    assert!(matches!(
        empty_url.validate(),
        Err(InitError::InvalidBaseUrl(_))
    ));

    // Invalid URL scheme
    let invalid_scheme = InitOptions::new("Test", "ftp://example.com");
    assert!(matches!(
        invalid_scheme.validate(),
        Err(InitError::InvalidBaseUrl(_))
    ));
}

#[test]
fn test_init_creates_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let new_site_path = temp_dir.path().join("new-site");

    let options = InitOptions::new("New Site", "https://new.example.com");
    let scaffolder = InitScaffolder::new(options);

    let report = scaffolder.scaffold(&new_site_path).unwrap();

    assert!(new_site_path.exists());
    assert!(new_site_path.join("site.toml").exists());
    assert_eq!(report.path, new_site_path);
}

#[test]
fn test_init_creates_scss_file() {
    let temp_dir = TempDir::new().unwrap();
    let options = InitOptions::new("Test Site", "https://test.example.com");
    let scaffolder = InitScaffolder::new(options);

    scaffolder.scaffold(temp_dir.path()).unwrap();

    let scss_path = temp_dir.path().join("styles/main.scss");
    assert!(scss_path.exists());

    let content = std::fs::read_to_string(scss_path).unwrap();
    assert!(content.contains("box-sizing"));
    assert!(content.contains("font-family"));
}
