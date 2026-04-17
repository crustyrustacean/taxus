// taxus-generator/src/build/pipeline/search.rs

use crate::build::ProcessedPage;
use crate::error::{GeneratorError, Result, SearchError};
use std::fs;
use std::path::Path;
use taxus_common::search::{SearchDocument, SearchIndex};
use tracing::debug;

// truncation limit, for summaries
const TRUNCATION_LIMIT: usize = 25;

#[derive(Clone, Debug)]
pub struct GeneratedSearch {
    pub search_index: Vec<u8>,
}

pub fn generate_search(processed_pages: &[ProcessedPage]) -> Result<GeneratedSearch> {
    let mut search_index = SearchIndex::new();

    for (id, processed) in processed_pages.iter().enumerate() {
        let new_search_document = SearchDocument::new(
            id as u32,
            processed.page.frontmatter.title.clone(),
            processed.route.path.clone(),
            truncate(&processed.page.summary(), TRUNCATION_LIMIT),
            processed.page.frontmatter.tags.clone(),
            processed.page.frontmatter.categories.clone(),
        );
        search_index.add_document(new_search_document, &processed.html_content);
    }

    search_index.finalize();

    let bytes = search_index
        .to_bytes()
        .map_err(|e| SearchError::SerializeFailed(e.to_string()))?;

    Ok(GeneratedSearch {
        search_index: bytes,
    })
}

pub fn write_search_index(
    generated_search: &GeneratedSearch,
    output_dir: &Path,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping search index write");
        return Ok(());
    }

    fs::create_dir_all(output_dir).map_err(|e| GeneratorError::Io {
        path: output_dir.to_path_buf(),
        source: e,
    })?;

    let output_path = output_dir.join("search_index.bin");

    fs::write(&output_path, &generated_search.search_index).map_err(|e| GeneratorError::Io {
        path: output_path.clone(),
        source: e,
    })?;

    debug!(
        path = %output_path.display(),
        "Written search index"
    );

    Ok(())
}

fn truncate(summary: &str, length: usize) -> String {
    let word_count = summary.split_whitespace().count();
    let truncated = summary
        .split_whitespace()
        .take(length)
        .collect::<Vec<&str>>()
        .join(" ");

    if word_count > length {
        format!("{}…", truncated)
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Frontmatter, Page};
    use crate::routes::{RouteInfo, RouteKind};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_processed_page(
        _id: usize,
        title: &str,
        content: &str,
        tags: Vec<String>,
    ) -> ProcessedPage {
        let route = RouteInfo::new(
            format!("/{}/", title.to_lowercase().replace(' ', "-")),
            PathBuf::from(format!("{}.md", title.to_lowercase().replace(' ', "-"))),
            PathBuf::from(format!(
                "{}/index.html",
                title.to_lowercase().replace(' ', "-")
            )),
            RouteKind::Page,
        )
        .unwrap();

        let page = Page {
            frontmatter: Frontmatter {
                title: title.to_string(),
                tags,
                ..Default::default()
            },
            path: route.path.clone(),
            source: route.content_file.clone(),
            raw_content: content.to_string(),
            content: None,
        };

        ProcessedPage {
            route,
            page,
            html_content: format!("<p>{}</p>", content),
            hero_image: None,
        }
    }

    #[test]
    fn generate_search_with_pages() {
        let processed = vec![
            make_processed_page(
                0,
                "Rust Guide",
                "Rust ownership borrowing lifetimes",
                vec!["rust".to_string()],
            ),
            make_processed_page(
                1,
                "Python Guide",
                "Python dynamic typing garbage collection",
                vec!["python".to_string()],
            ),
        ];

        let result = generate_search(&processed);
        assert!(result.is_ok());

        let generated = result.unwrap();
        assert!(!generated.search_index.is_empty());

        // Verify we can deserialize and search
        let index = SearchIndex::from_bytes(&generated.search_index).unwrap();
        assert_eq!(index.documents.len(), 2);

        let results = index.search("rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Guide");
    }

    #[test]
    fn generate_search_with_empty_pages() {
        let processed: Vec<ProcessedPage> = vec![];

        let result = generate_search(&processed);
        assert!(result.is_ok());

        let generated = result.unwrap();
        let index = SearchIndex::from_bytes(&generated.search_index).unwrap();
        assert_eq!(index.documents.len(), 0);
    }

    #[test]
    fn write_search_index_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let generated = GeneratedSearch {
            search_index: vec![1, 2, 3, 4],
        };

        let result = write_search_index(&generated, &output_dir, false);
        assert!(result.is_ok());
        assert!(output_dir.join("search_index.bin").exists());

        let written = fs::read(output_dir.join("search_index.bin")).unwrap();
        assert_eq!(written, vec![1, 2, 3, 4]);
    }

    #[test]
    fn write_search_index_dry_run_does_not_write() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");

        let generated = GeneratedSearch {
            search_index: vec![1, 2, 3, 4],
        };

        let result = write_search_index(&generated, &output_dir, true);
        assert!(result.is_ok());
        assert!(!output_dir.join("search_index.bin").exists());
    }

    #[test]
    fn generate_and_write_round_trip() {
        let processed = vec![
            make_processed_page(
                0,
                "First Post",
                "Rust async programming futures",
                vec!["rust".to_string()],
            ),
            make_processed_page(
                1,
                "Second Post",
                "Python web frameworks django flask",
                vec!["python".to_string()],
            ),
            make_processed_page(
                2,
                "Third Post",
                "Rust web frameworks actix axum",
                vec!["rust".to_string()],
            ),
        ];

        let generated = generate_search(&processed).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("dist");
        write_search_index(&generated, &output_dir, false).unwrap();

        // Read back from disk and verify
        let bytes = fs::read(output_dir.join("search_index.bin")).unwrap();
        let index = SearchIndex::from_bytes(&bytes).unwrap();

        assert_eq!(index.documents.len(), 3);

        let results = index.search("rust");
        assert_eq!(results.len(), 2);

        let results = index.search("django");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Second Post");

        let results = index.search("bananas");
        assert_eq!(results.len(), 0);
    }
}
