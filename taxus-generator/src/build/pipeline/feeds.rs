// taxus-generator/src/build/pipeline/generate_feeds.rs

use crate::build::ProcessedPage;
use crate::config::SiteConfig;
use crate::error::{BuildError, Result};
use std::fs;
use std::path::Path;
use tracing::{debug, info};

/// Generated feed file.
#[derive(Debug, Clone)]
pub struct GeneratedFeed {
    /// Feed filename (e.g., "feed.xml", "atom.xml")
    pub filename: String,
    /// Feed content (XML)
    pub content: String,
}

/// Generate RSS and Atom feeds from processed pages.
pub fn generate_feeds(
    processed: &[ProcessedPage],
    config: &SiteConfig,
) -> Result<Vec<GeneratedFeed>> {
    use crate::feed::{FeedConfig as FeedGenConfig, FeedGenerator};

    let mut feeds = Vec::new();

    // Skip if both feeds are disabled
    if !config.feed.rss_enabled && !config.feed.atom_enabled {
        return Ok(feeds);
    }

    // Collect pages for feed generation
    let pages: Vec<crate::content::Page> = processed
        .iter()
        .filter(|p| !p.page.is_draft()) // Exclude drafts from feeds
        .map(|p| {
            let mut page = p.page.clone();
            // Update the page path to use the correct URL path
            let url_path = if p.page.frontmatter.slug.is_some() {
                p.page.url_path()
            } else {
                p.route.path.clone()
            };
            page.path = url_path;
            // Set content for full-content feeds
            if config.feed.full_content {
                page.content = Some(p.html_content.clone());
            }
            page
        })
        .collect();

    // Build feed generator config
    let feed_gen_config = FeedGenConfig {
        title: config
            .feed
            .title
            .clone()
            .unwrap_or_else(|| config.site.name.clone()),
        base_url: config.site.base_url.clone(),
        description: config.site.description.clone().unwrap_or_default(),
        author: config.site.author.clone(),
        limit: if config.feed.limit > 0 {
            config.feed.limit
        } else {
            20
        },
        full_content: config.feed.full_content,
        ..Default::default()
    };

    let generator = FeedGenerator::new(feed_gen_config);

    // Generate RSS feed if enabled
    if config.feed.rss_enabled {
        let rss_content = generator.generate_rss(&pages)?;
        let filename = config
            .feed
            .rss_path
            .clone()
            .unwrap_or_else(|| generator.rss_filename());
        feeds.push(GeneratedFeed {
            filename,
            content: rss_content,
        });
        info!("Generated RSS feed");
    }

    // Generate Atom feed if enabled
    if config.feed.atom_enabled {
        let atom_content = generator.generate_atom(&pages)?;
        let filename = config
            .feed
            .atom_path
            .clone()
            .unwrap_or_else(|| generator.atom_filename());
        feeds.push(GeneratedFeed {
            filename,
            content: atom_content,
        });
        info!("Generated Atom feed");
    }

    Ok(feeds)
}

/// Write feed files to output directory.
pub fn write_feeds(feeds: &[GeneratedFeed], output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping feed writes");
        return Ok(());
    }

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).map_err(|e| BuildError::Io {
        path: output_dir.to_path_buf(),
        source: e,
    })?;

    for feed in feeds {
        let output_path = output_dir.join(&feed.filename);

        // Write the feed file
        fs::write(&output_path, &feed.content).map_err(|e| BuildError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        debug!(
            path = %output_path.display(),
            "Written feed file"
        );
    }

    if !feeds.is_empty() {
        info!("Wrote {} feed files", feeds.len());
    }

    Ok(())
}

#[test]
fn test_write_feeds_creates_output_dir() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    // Use a non-existent output directory
    let output_dir = temp_dir.path().join("dist");

    // Ensure output directory doesn't exist
    assert!(!output_dir.exists());

    let feeds = vec![GeneratedFeed {
        filename: "feed.xml".to_string(),
        content: r#"<?xml version="1.0" encoding="UTF-8"?><rss></rss>"#.to_string(),
    }];

    // Write feeds - should create the directory
    let result = write_feeds(&feeds, &output_dir, false);
    assert!(result.is_ok());

    // Verify directory was created
    assert!(output_dir.exists());

    // Verify file was written
    assert!(output_dir.join("feed.xml").exists());
}

#[test]
fn test_write_feeds_dry_run() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    let feeds = vec![GeneratedFeed {
        filename: "feed.xml".to_string(),
        content: r#"<?xml version="1.0" encoding="UTF-8"?><rss></rss>"#.to_string(),
    }];

    // Dry run should not write anything
    let result = write_feeds(&feeds, &output_dir, true);
    assert!(result.is_ok());

    // Verify directory was NOT created
    assert!(!output_dir.exists());
}

#[test]
fn test_write_feeds_empty() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    // Empty feeds list should succeed
    let result = write_feeds(&[], &output_dir, false);
    assert!(result.is_ok());
}
