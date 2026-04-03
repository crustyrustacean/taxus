// generator/src/build/pipeline/not_found.rs

use crate::error::{BuildError, Result};
use crate::templates::{SiteContext, TemplateContext, TemplateRenderer, TeraRenderer};
use std::fs;
use std::path::Path;
use tracing::{debug, info};

/// Generated 404.html content.
#[derive(Debug, Clone)]
pub struct Generated404 {
    /// HTML content
    pub content: String,
}

/// Generate 404.html page.
///
/// If 404.html template doesn't exist, returns None.
/// Otherwise, renders the template with site context.
pub fn generate_404(
    templates: &TeraRenderer,
    site_context: &SiteContext,
) -> Result<Option<Generated404>> {
    if !templates.has_template("404.html") {
        debug!("No 404.html template found, skipping 404 page generation");
        return Ok(None);
    }

    let context = TemplateContext::new(site_context.clone());
    let content = templates.render("404.html", &context)?;

    info!("Generated 404.html");
    Ok(Some(Generated404 { content }))
}

/// Write 404.html to output directory.
pub fn write_404(page: &Generated404, output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping 404.html write");
        return Ok(());
    }

    fs::create_dir_all(output_dir).map_err(|e| BuildError::Io {
        path: output_dir.to_path_buf(),
        source: e,
    })?;

    let output_path = output_dir.join("404.html");

    fs::write(&output_path, &page.content).map_err(|e| BuildError::Io {
        path: output_path.clone(),
        source: e,
    })?;

    debug!(
        path = %output_path.display(),
        "Written 404.html"
    );

    info!("Wrote 404.html");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::SiteContext;
    use tempfile::TempDir;

    fn create_test_site_context() -> SiteContext {
        SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: Some("A test site".to_string()),
            author: Some("Test Author".to_string()),
        }
    }

    #[test]
    fn test_generate_404_returns_some_when_template_exists() {
        let mut renderer = TeraRenderer::new().unwrap();
        renderer
            .register_template("404.html", "<html><body>Not Found</body></html>")
            .unwrap();

        let site_context = create_test_site_context();
        let result = generate_404(&renderer, &site_context);

        assert!(result.is_ok());
        let generated = result.unwrap();
        assert!(generated.is_some());
        let generated = generated.unwrap();
        assert!(generated.content.contains("Not Found"));
    }

    #[test]
    fn test_generate_404_returns_none_when_no_template() {
        let renderer = TeraRenderer::new().unwrap();
        let site_context = create_test_site_context();
        let result = generate_404(&renderer, &site_context);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_write_404_writes_file_when_not_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        assert!(!output_dir.exists());

        let page = Generated404 {
            content: "<html><body>Not Found</body></html>".to_string(),
        };

        let result = write_404(&page, &output_dir, false);
        assert!(result.is_ok());

        assert!(output_dir.exists());
        let file_path = output_dir.join("404.html");
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Not Found"));
    }

    #[test]
    fn test_write_404_does_not_write_when_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let page = Generated404 {
            content: "<html><body>Not Found</body></html>".to_string(),
        };

        let result = write_404(&page, &output_dir, true);
        assert!(result.is_ok());

        assert!(!output_dir.exists());
    }
}
