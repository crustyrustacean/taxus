//! Integration tests for configuration loading.

use std::path::PathBuf;
use yew_ssg_lib::config::SiteConfig;
use yew_ssg_lib::error::{ConfigError, GeneratorError};

#[test]
fn test_load_minimal_site_config() {
    let result = SiteConfig::from_dir("tests/fixtures/minimal_site");

    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.site.name, "Minimal Site");
    assert_eq!(config.site.base_url, "https://minimal.example.com");
    assert!(config.site.description.is_none());
    assert!(config.site.author.is_none());

    // Check defaults are applied
    assert_eq!(config.build.content_dir, PathBuf::from("content"));
    assert_eq!(config.build.output_dir, PathBuf::from("dist"));
}

#[test]
fn test_load_full_site_config() {
    let result = SiteConfig::from_dir("tests/fixtures/full_site");

    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.site.name, "Full Site");
    assert_eq!(config.site.base_url, "https://full.example.com");
    assert_eq!(
        config.site.description,
        Some("A fully configured site".to_string())
    );
    assert_eq!(config.site.author, Some("Test Author".to_string()));

    assert_eq!(config.build.content_dir, PathBuf::from("content"));
    assert_eq!(config.build.output_dir, PathBuf::from("dist"));
}

#[test]
fn test_load_missing_config() {
    let result = SiteConfig::from_dir("tests/fixtures/nonexistent");

    assert!(result.is_err());
    let err = result.unwrap_err();

    match err {
        GeneratorError::Config(ConfigError::NotFound(path)) => {
            assert!(path.ends_with("site.toml"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_load_from_file() {
    let result = SiteConfig::from_file("tests/fixtures/minimal_site/site.toml");

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.site.name, "Minimal Site");
}

#[test]
fn test_validate_config() {
    let config = SiteConfig::from_dir("tests/fixtures/full_site").unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_new_constructor() {
    let config = SiteConfig::new("New Site", "https://new.example.com");

    assert_eq!(config.site.name, "New Site");
    assert_eq!(config.site.base_url, "https://new.example.com");
    assert!(config.validate().is_ok());
}
