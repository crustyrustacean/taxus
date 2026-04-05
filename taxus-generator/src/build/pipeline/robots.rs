// generator/src/build/pipeline/robots.rs

// dependencies
use crate::config::SiteConfig;
use crate::error::{GeneratorError, Result};
use std::fs;
use std::path::Path;
use tracing::{debug, info};

/// Generated robots.txt content.
#[derive(Debug, Clone)]
pub struct GeneratedRobots {
    /// Robots.txt content
    pub content: String,
}

/// Generate robots.txt content.
///
/// If a robots.txt already exists in the static directory, returns None
/// (the existing file will be copied by StaticCopier). Otherwise, generates
/// a default robots.txt with sitemap reference.
pub fn generate_robots(config: &SiteConfig) -> Result<Option<GeneratedRobots>> {
    // Check if static/robots.txt already exists
    let static_robots = config.build.static_dir.join("robots.txt");
    if static_robots.exists() {
        debug!(
            path = %static_robots.display(),
            "Static robots.txt exists, skipping generation"
        );
        return Ok(None);
    }

    // Generate default robots.txt
    let base_url = &config.site.base_url;
    let content = format!(
        r#"User-agent: *
Allow: /

Sitemap: {}/sitemap.xml
"#,
        base_url.trim_end_matches('/')
    );

    info!("Generated default robots.txt");
    Ok(Some(GeneratedRobots { content }))
}

/// Write robots.txt to output directory.
pub fn write_robots(robots: &GeneratedRobots, output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping robots.txt write");
        return Ok(());
    }

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).map_err(|e| GeneratorError::Io {
        path: output_dir.to_path_buf(),
        source: e,
    })?;

    let output_path = output_dir.join("robots.txt");

    // Write the robots.txt file
    fs::write(&output_path, &robots.content).map_err(|e| GeneratorError::Io {
        path: output_path.clone(),
        source: e,
    })?;

    debug!(
        path = %output_path.display(),
        "Written robots.txt"
    );

    info!("Wrote robots.txt");
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_write_robots_creates_output_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        // Use a non-existent output directory
        let output_dir = temp_dir.path().join("dist");

        // Ensure output directory doesn't exist
        assert!(!output_dir.exists());

        let robots = GeneratedRobots {
            content: "User-agent: *\nAllow: /\n".to_string(),
        };

        // Write robots.txt - should create the directory
        let result = write_robots(&robots, &output_dir, false);
        assert!(result.is_ok());

        // Verify directory was created
        assert!(output_dir.exists());

        // Verify file was written
        assert!(output_dir.join("robots.txt").exists());
    }

    #[test]
    fn test_write_robots_dry_run() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let robots = GeneratedRobots {
            content: "User-agent: *\nAllow: /\n".to_string(),
        };

        // Dry run should not write anything
        let result = write_robots(&robots, &output_dir, true);
        assert!(result.is_ok());

        // Verify directory was NOT created
        assert!(!output_dir.exists());
    }
}
