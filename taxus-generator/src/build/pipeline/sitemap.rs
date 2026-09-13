// generator/src/build/pipeline/sitemap.rs

//! Stage 10 (analyse, emit): `sitemap.xml`.
//!
//! The URL set composes from final outputs (#47): every `RenderedPage`
//! — which includes the pagination pages stage 6 emits — plus every
//! taxonomy list and term page from stage 9. Alias redirects are
//! excluded deliberately: they are redirects, not content — each
//! targets a URL already in the set. `<loc>` is XML-escaped (#47).
//! See the book's
//! [Derivations](https://crustyrustacean.github.io/taxus/theory/derivations.html) chapter.

use crate::build::ProcessedPage;
use crate::build::RenderedPage;
use crate::build::pipeline::taxonomy::RenderedTaxonomy;
use crate::config::SiteConfig;
use crate::error::{GeneratorError, Result};
use crate::feed::escape_xml;
use crate::templates::compute_permalink;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, info};

/// Sitemap URL entry.
#[derive(Debug, Clone)]
pub struct SitemapUrl {
    /// Full URL (base_url + path)
    pub loc: String,
    /// Last modification date (YYYY-MM-DD format)
    pub lastmod: Option<String>,
    /// Change frequency
    pub changefreq: String,
    /// Priority (0.0 to 1.0)
    pub priority: String,
}

/// Generated sitemap.xml content.
#[derive(Debug, Clone)]
pub struct GeneratedSitemap {
    /// Sitemap XML content
    pub content: String,
    /// Number of URLs in the sitemap
    pub url_count: usize,
}

/// Generate sitemap.xml.
///
/// Creates a sitemap with:
/// - Every rendered page (pages, sections and the pagination pages
///   `render_pages` emits for paginated sections)
/// - Every taxonomy list and term page (`/tags/`, `/tags/rust/`, …)
/// - lastmod from page date if available
/// - Priority: 1.0 for home, 0.8 for sections and taxonomy pages, 0.7 for pages
/// - changefreq: weekly for home, monthly for others
///
/// Alias redirect pages are excluded deliberately (#47): they redirect
/// to URLs already present.
pub fn generate_sitemap(
    rendered: &[RenderedPage],
    taxonomy_pages: &[RenderedTaxonomy],
    processed: &[ProcessedPage],
    config: &SiteConfig,
) -> Result<GeneratedSitemap> {
    let base_url = config.site.base_url.trim_end_matches('/');
    let mut urls: Vec<SitemapUrl> = Vec::new();

    // Dates come from the processed pages, joined by content file —
    // `RenderedPage` carries only what writing needs.
    let date_by_file: HashMap<&Path, String> = processed
        .iter()
        .filter_map(|p| {
            p.page
                .frontmatter
                .date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .map(|d| (p.route.content_file.as_path(), d))
        })
        .collect();

    // Pages, sections and pagination — stage 6's full output.
    for page in rendered {
        let lastmod = date_by_file.get(page.route.content_file.as_path()).cloned();
        urls.push(SitemapUrl {
            loc: compute_permalink(base_url, &page.route.path),
            lastmod,
            changefreq: if page.route.path == "/" {
                "weekly".to_string()
            } else {
                "monthly".to_string()
            },
            priority: if page.route.path == "/" {
                "1.0".to_string()
            } else if page.route.is_section() {
                "0.8".to_string()
            } else {
                "0.7".to_string()
            },
        });
    }

    // Taxonomy list and term pages — stage 9's output.
    for taxonomy in taxonomy_pages {
        urls.push(SitemapUrl {
            loc: compute_permalink(base_url, &taxonomy.path),
            lastmod: None,
            changefreq: "monthly".to_string(),
            priority: "0.8".to_string(),
        });
    }

    // Sort URLs by path for consistent output.
    urls.sort_by(|a, b| a.loc.cmp(&b.loc));

    // Generate XML
    let mut xml = String::new();
    xml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
    );

    for url in &urls {
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", escape_xml(&url.loc)));
        if let Some(ref lastmod) = url.lastmod {
            xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
        }
        xml.push_str(&format!(
            "    <changefreq>{}</changefreq>\n",
            url.changefreq
        ));
        xml.push_str(&format!("    <priority>{}</priority>\n", url.priority));
        xml.push_str("  </url>\n");
    }

    xml.push_str("</urlset>\n");

    let url_count = urls.len();
    info!("Generated sitemap.xml with {} URLs", url_count);

    Ok(GeneratedSitemap {
        content: xml,
        url_count,
    })
}

/// Write sitemap.xml to output directory.
pub fn write_sitemap(sitemap: &GeneratedSitemap, output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping sitemap.xml write");
        return Ok(());
    }

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).map_err(|e| GeneratorError::Io {
        path: output_dir.to_path_buf(),
        source: e,
    })?;

    let output_path = output_dir.join("sitemap.xml");

    // Write the sitemap file
    fs::write(&output_path, &sitemap.content).map_err(|e| GeneratorError::Io {
        path: output_path.clone(),
        source: e,
    })?;

    debug!(
        path = %output_path.display(),
        urls = sitemap.url_count,
        "Written sitemap.xml"
    );

    info!("Wrote sitemap.xml with {} URLs", sitemap.url_count);
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_write_sitemap_creates_output_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        // Use a non-existent output directory
        let output_dir = temp_dir.path().join("dist");

        // Ensure output directory doesn't exist
        assert!(!output_dir.exists());

        let sitemap = GeneratedSitemap {
            content: r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
</urlset>"#
                .to_string(),
            url_count: 0,
        };

        // Write sitemap.xml - should create the directory
        let result = write_sitemap(&sitemap, &output_dir, false);
        assert!(result.is_ok());

        // Verify directory was created
        assert!(output_dir.exists());

        // Verify file was written
        assert!(output_dir.join("sitemap.xml").exists());
    }

    #[test]
    fn test_write_sitemap_dry_run() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let sitemap = GeneratedSitemap {
            content: r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string(),
            url_count: 0,
        };

        // Dry run should not write anything
        let result = write_sitemap(&sitemap, &output_dir, true);
        assert!(result.is_ok());

        // Verify directory was NOT created
        assert!(!output_dir.exists());
    }
}
