// taxus-generator/src/build/pipeline/search.rs

//! Stage 13 (emit): the client-side search index, `search_index.bin`.
//!
//! One `SearchDocument` per processed page, in registry (tree) order,
//! with the served URL path from the tree. The indexed *text* is the
//! page's Markdown body — not its rendered HTML — plus its title
//! (repeated, a crude field boost) and tags/categories (#43): rendered
//! HTML fills the term space with tag names, attribute names and
//! highlighter classes, and a query matching only the title used to
//! return nothing. See the book's
//! [Search](https://crustyrustacean.github.io/taxus/search.html) chapter.

use crate::build::ProcessedPage;
use crate::build::pipeline::markdown::markdown_text;
use crate::error::{GeneratorError, Result, SearchError};
use std::fs;
use std::path::Path;
use taxus_common::search::{SearchDocument, SearchIndex};
use tracing::debug;

// truncation limit, for summaries
const TRUNCATION_LIMIT: usize = 25;

/// How many times the title (and each tag/category) is repeated in the
/// indexed text, raising its term frequency relative to the body.
///
/// A crude field boost — but TF-IDF only understands frequencies, and
/// title matches are what the visitor most often means.
const TITLE_BOOST: usize = 3;

#[derive(Clone, Debug)]
pub struct GeneratedSearch {
    pub search_index: Vec<u8>,
}

/// The text indexed for a page: its Markdown words, its title, and its
/// taxonomy terms, the latter repeated as a field boost.
pub fn indexed_text(processed: &ProcessedPage) -> String {
    let mut text = String::new();
    for _ in 0..TITLE_BOOST {
        text.push_str(&processed.page.frontmatter.title);
        text.push(' ');
    }
    for term in processed
        .page
        .frontmatter
        .tags
        .iter()
        .chain(processed.page.frontmatter.categories.iter())
    {
        for _ in 0..TITLE_BOOST {
            text.push_str(term);
            text.push(' ');
        }
    }
    text.push_str(&markdown_text(&processed.page.raw_content));
    text
}

pub fn generate_search(processed_pages: &[ProcessedPage]) -> Result<GeneratedSearch> {
    let mut search_index = SearchIndex::new();

    for (id, processed) in processed_pages.iter().enumerate() {
        let new_search_document = SearchDocument::new(
            id as u32,
            processed.page.frontmatter.title.clone(),
            // #25: the effective URL, not route.path — a custom slug
            // moves the page, and search results must follow it or they
            // navigate to a 404.
            processed.effective_url_path(),
            truncate(&processed.page.summary(), TRUNCATION_LIMIT),
            processed.page.frontmatter.tags.clone(),
            processed.page.frontmatter.categories.clone(),
        );
        search_index.add_document(new_search_document, &indexed_text(processed));
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
            raw_content: content.to_string(),
        };

        ProcessedPage {
            toc: Vec::new(),
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
