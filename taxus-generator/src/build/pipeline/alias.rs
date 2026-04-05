// taxus-generator/src/pipeline/alias.rs

use crate::error::{GeneratorError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Alias page for redirects.
#[derive(Debug, Clone)]
pub struct AliasPage {
    /// Alias URL path (e.g., "/old-url/")
    pub alias_path: String,
    /// Target URL path (e.g., "/new-url/")
    pub target_path: String,
    /// Output file path for the redirect page
    pub output_file: PathBuf,
}

impl AliasPage {
    /// Create a new alias page.
    pub fn new(alias_path: String, target_path: String) -> Self {
        // Convert alias path to output file path
        // "/old-url/" -> "old-url/index.html"
        let output_file = if alias_path == "/" {
            PathBuf::from("index.html")
        } else {
            let trimmed = alias_path.trim_start_matches('/').trim_end_matches('/');
            if trimmed.is_empty() {
                PathBuf::from("index.html")
            } else {
                PathBuf::from(trimmed).join("index.html")
            }
        };

        Self {
            alias_path,
            target_path,
            output_file,
        }
    }

    /// Generate HTML redirect page.
    pub fn to_html(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0;url={}">
    <link rel="canonical" href="{}">
    <title>Redirecting...</title>
</head>
<body>
    <p>Redirecting to <a href="{}">{}</a>...</p>
</body>
</html>"#,
            self.target_path, self.target_path, self.target_path, self.target_path
        )
    }
}

/// Write alias redirect pages.
pub fn write_aliases(aliases: &[AliasPage], output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping alias file writes");
        return Ok(());
    }

    for alias in aliases {
        let output_path = output_dir.join(&alias.output_file);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| GeneratorError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write the redirect HTML
        let html = alias.to_html();
        fs::write(&output_path, &html).map_err(|e| GeneratorError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        debug!(
            path = %output_path.display(),
            alias = %alias.alias_path,
            target = %alias.target_path,
            "Written alias redirect file"
        );
    }

    info!("Wrote {} alias redirects", aliases.len());
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_alias_page_new() {
        let alias = AliasPage::new("/old-url/".to_string(), "/new-url/".to_string());
        assert_eq!(alias.alias_path, "/old-url/");
        assert_eq!(alias.target_path, "/new-url/");
        assert_eq!(alias.output_file, PathBuf::from("old-url/index.html"));
    }

    #[test]
    fn test_alias_page_root() {
        let alias = AliasPage::new("/".to_string(), "/new-home/".to_string());
        assert_eq!(alias.alias_path, "/");
        assert_eq!(alias.target_path, "/new-home/");
        assert_eq!(alias.output_file, PathBuf::from("index.html"));
    }

    #[test]
    fn test_alias_page_to_html() {
        let alias = AliasPage::new("/old-url/".to_string(), "/new-url/".to_string());
        let html = alias.to_html();

        // Check that the HTML contains the redirect elements
        assert!(html.contains(r#"http-equiv="refresh""#));
        assert!(html.contains("0;url=/new-url/"));
        assert!(html.contains(r#"rel="canonical""#));
        assert!(html.contains("href=\"/new-url/\""));
        assert!(html.contains("<a href=\"/new-url/\""));
    }

    #[test]
    fn test_alias_page_deep_path() {
        let alias = AliasPage::new("/blog/old-post/".to_string(), "/blog/new-post/".to_string());
        assert_eq!(alias.alias_path, "/blog/old-post/");
        assert_eq!(alias.target_path, "/blog/new-post/");
        assert_eq!(alias.output_file, PathBuf::from("blog/old-post/index.html"));
    }
}
