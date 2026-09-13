// taxus-common/src/search.rs

//! The client-side TF-IDF search index (#22, #43, #57).
//!
//! The index is built by the generator (from a page's Markdown text and
//! its title/tags/categories — never its rendered HTML, which would fill
//! the index with tag names and highlighter classes) and searched by the
//! WASM client. Both run on the same types, in WASM and on the host.

use postcard::{from_bytes, to_allocvec};
use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

// The English stemmer, constructed once per thread (#57).
//
// `Stemmer::create` is cheap but sits on the hot path — once per
// document at index time and once per query at search time. A
// `thread_local!` is the only hoist that works everywhere this crate
// compiles (host and WASM alike); `OnceLock` in a `static` would work
// on the host but not under `wasm-bindgen`'s threaded runtime rules.
thread_local! {
    static ENGLISH_STEMMER: Stemmer = Stemmer::create(Algorithm::English);
}

// struct type to represent the metadata record stored for each indexed page
#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct SearchDocument {
    pub id: u32,
    pub title: String,
    pub path: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}

impl SearchDocument {
    pub fn new(
        id: u32,
        title: String,
        path: String,
        summary: String,
        tags: Vec<String>,
        categories: Vec<String>,
    ) -> Self {
        Self {
            id,
            title,
            path,
            summary,
            tags,
            categories,
        }
    }
}

// struct type to represent the Search Index
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SearchIndex {
    pub documents: BTreeMap<u32, SearchDocument>,
    pub index: HashMap<String, Vec<(u32, f32)>>,
}

/// How many results [`SearchIndex::search`] returns at most (#22).
pub const SEARCH_RESULT_LIMIT: usize = 10;

impl SearchIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_document(&mut self, search_document: SearchDocument, content: &str) {
        let document_id = search_document.id;
        self.documents.insert(document_id, search_document);
        let tokens = tokenize(content);
        let stems = stem(&tokens);

        // Count occurrences of each stem
        let mut stem_counts: HashMap<String, u32> = HashMap::new();
        for stem in stems.iter() {
            *stem_counts.entry(stem.to_string()).or_insert(0) += 1;
        }

        // Store counts in the index (will be converted to TF-IDF later)
        let total_words = stems.len() as f32;
        for (stem, count) in stem_counts.iter() {
            let tf = *count as f32 / total_words;
            self.index
                .entry(stem.to_string())
                .or_default()
                .push((document_id, tf));
        }
    }

    /// Search the index, returning at most [`SEARCH_RESULT_LIMIT`] results
    /// in descending score order.
    ///
    /// A cap, not a score threshold: TF-IDF scores have no corpus-independent
    /// floor — on a three-page site every match is a good match — so a
    /// threshold that works on one site suppresses valid results on another
    /// (#22). The cap keeps noise out of large corpora while never hiding a
    /// relevant hit on a small one.
    pub fn search(&self, query: &str) -> Vec<&SearchDocument> {
        self.search_with_limit(query, SEARCH_RESULT_LIMIT)
    }

    /// [`search`](Self::search) with a caller-chosen result cap.
    pub fn search_with_limit(&self, query: &str, limit: usize) -> Vec<&SearchDocument> {
        let tokens = tokenize(query);
        let stems = stem(&tokens);

        let mut scores: HashMap<u32, f32> = HashMap::new();
        for stem in stems.iter() {
            if let Some(value) = self.index.get(stem) {
                for item in value.iter() {
                    *scores.entry(item.0).or_insert(0.0) += item.1;
                }
            }
        }

        // `total_cmp` orders NaN safely where `partial_cmp().unwrap()`
        // would panic (#57). Scores of exactly 0 are kept deliberately:
        // on a two-document corpus a term in both documents has IDF
        // ln(2/2) = 0, and hiding every match for it would punish small
        // sites — the cap, not a threshold, is the noise control.
        let mut results: Vec<(u32, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.total_cmp(&a.1));

        results
            .iter()
            .take(limit)
            .filter_map(|(id, _score)| self.documents.get(id))
            .collect()
    }

    pub fn finalize(&mut self) {
        let total_docs = self.documents.len() as f32;

        for entries in self.index.values_mut() {
            let docs_with_term = entries.len() as f32;
            let idf = (total_docs / docs_with_term).ln();

            for entry in entries.iter_mut() {
                entry.1 *= idf;
            }
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        to_allocvec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        from_bytes(bytes)
    }
}

// tokenizer
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .map(|s| s.to_string())
        .filter(|s| s.len() > 2)
        .collect::<Vec<String>>()
}

// stemmer
pub fn stem(tokens: &[String]) -> Vec<String> {
    ENGLISH_STEMMER.with(|stemmer| {
        tokens
            .iter()
            .map(|t| stemmer.stem(t).to_string())
            .collect::<Vec<String>>()
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_new_search_document_works() {
        let test_document = SearchDocument::new(
            1,
            "Test Title".to_string(),
            "test_path".to_string(),
            "this is the test summary".to_string(),
            vec!["tag".to_string()],
            vec!["category".to_string()],
        );

        assert_eq!(test_document.id, 1);
        assert_eq!(test_document.title, "Test Title");
        assert_eq!(test_document.path, "test_path");
        assert_eq!(test_document.summary, "this is the test summary");
        assert_eq!(test_document.tags, vec!["tag"]);
        assert_eq!(test_document.categories, vec!["category"]);
    }

    #[test]
    fn test_tokenizer_output() {
        let result = tokenize("Rust's async/await is powerful!");
        assert_eq!(result, vec!["rust", "async", "await", "powerful"]);
    }

    #[test]
    fn test_tokenizer_with_empty_string_returns_empty_vec() {
        let result = tokenize("");
        let expected: Vec<String> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_tokenizer_with_punctuation_returns_empty_vec() {
        let result = tokenize("!@#$%");
        let expected: Vec<String> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_tokenizer_with_short_word_returns_empty_vec() {
        let result = tokenize("I");
        let expected: Vec<String> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_tokenizer_with_single_word_returns_vec() {
        let result = tokenize("rust");
        assert_eq!(result, vec!["rust"]);
    }

    #[test]
    fn test_tokenizer_with_single_accented_word_returns_vec() {
        let result = tokenize("café");
        assert_eq!(result, vec!["café"]);
    }
    #[test]
    fn test_stem_basic() {
        let tokens = vec!["programming".to_string(), "programs".to_string()];
        let result = stem(&tokens);
        assert_eq!(result[0], result[1]);
    }

    #[test]
    fn test_stem_empty() {
        let result = stem(&[]);
        let expected: Vec<String> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn create_new_search_index_works() {
        let search_index = SearchIndex::new();
        assert_eq!(search_index.documents, BTreeMap::new());
        assert_eq!(search_index.index, HashMap::new());
    }

    #[test]
    fn add_document_to_search_index_works() {
        let mut search_index = SearchIndex::new();
        let doc1 = SearchDocument::new(
            1,
            "Test Title".to_string(),
            "test_path".to_string(),
            "this is the test summary".to_string(),
            vec!["tag".to_string()],
            vec!["category".to_string()],
        );
        let doc2 = SearchDocument::new(
            2,
            "Second Title".to_string(),
            "second_path".to_string(),
            "this is the second summary".to_string(),
            vec!["tag".to_string()],
            vec!["category".to_string()],
        );

        search_index.add_document(doc1, "Rust is a systems programming language.");
        search_index.add_document(doc2, "Rust async programming is fast.");

        assert_eq!(search_index.documents.len(), 2);
        assert_eq!(search_index.index.len(), 6);

        // "rust" and "program" appear in both documents
        assert_eq!(search_index.index["rust"].len(), 2);
        assert_eq!(search_index.index["program"].len(), 2);

        // these appear in only one document
        assert_eq!(search_index.index["system"].len(), 1);
        assert_eq!(search_index.index["languag"].len(), 1);
        assert_eq!(search_index.index["async"].len(), 1);
        assert_eq!(search_index.index["fast"].len(), 1);
    }

    #[test]
    fn add_document_with_empty_content() {
        let mut search_index = SearchIndex::new();
        let doc = SearchDocument::new(
            1,
            "Empty Page".to_string(),
            "/empty/".to_string(),
            "no content".to_string(),
            vec![],
            vec![],
        );

        search_index.add_document(doc, "");

        assert_eq!(search_index.documents.len(), 1);
        assert_eq!(search_index.index.len(), 0);
    }

    #[test]
    fn add_document_with_duplicate_words() {
        let mut search_index = SearchIndex::new();
        let doc = SearchDocument::new(
            1,
            "Repeat Page".to_string(),
            "/repeat/".to_string(),
            "summary".to_string(),
            vec![],
            vec![],
        );

        search_index.add_document(doc, "rust rust rust");

        assert_eq!(search_index.documents.len(), 1);
        assert_eq!(search_index.index["rust"].len(), 1);
    }

    #[test]
    fn add_document_with_no_meaningful_words() {
        let mut search_index = SearchIndex::new();
        let doc = SearchDocument::new(
            1,
            "Noise Page".to_string(),
            "/noise/".to_string(),
            "summary".to_string(),
            vec![],
            vec![],
        );

        search_index.add_document(doc, "a I & ! @ #");

        assert_eq!(search_index.documents.len(), 1);
        assert_eq!(search_index.index.len(), 0);
    }

    #[test]
    fn search_with_two_matches() {
        let mut search_index = SearchIndex::new();
        search_index.add_document(
            SearchDocument::new(
                0,
                "First".to_string(),
                "/first/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "Rust is a systems programming language",
        );
        search_index.add_document(
            SearchDocument::new(
                1,
                "Second".to_string(),
                "/second/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "Rust async programming is fast",
        );
        search_index.finalize();
        let results = search_index.search("rust");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_with_one_match() {
        let mut search_index = SearchIndex::new();
        search_index.add_document(
            SearchDocument::new(
                0,
                "First".to_string(),
                "/first/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "Rust is a systems programming language",
        );
        search_index.add_document(
            SearchDocument::new(
                1,
                "Second".to_string(),
                "/second/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "Rust async programming is fast",
        );
        search_index.finalize();
        let results = search_index.search("fast");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Second");
    }

    #[test]
    fn search_with_no_matches() {
        let mut search_index = SearchIndex::new();
        search_index.add_document(
            SearchDocument::new(
                0,
                "First".to_string(),
                "/first/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "Rust is a systems programming language",
        );
        search_index.finalize();
        let results = search_index.search("bananas");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn search_index_round_trip_serialization() {
        let mut original = SearchIndex::new();
        original.add_document(
            SearchDocument::new(
                0,
                "First".to_string(),
                "/first/".to_string(),
                "summary one".to_string(),
                vec!["rust".to_string()],
                vec![],
            ),
            "Rust is a systems programming language",
        );
        original.add_document(
            SearchDocument::new(
                1,
                "Second".to_string(),
                "/second/".to_string(),
                "summary two".to_string(),
                vec![],
                vec![],
            ),
            "Rust async programming is fast",
        );
        original.finalize();

        let bytes = original.to_bytes().unwrap();
        let restored = SearchIndex::from_bytes(&bytes).unwrap();

        // Same number of documents
        assert_eq!(restored.documents.len(), 2);
        assert_eq!(restored.documents[&0].title, "First");
        assert_eq!(restored.documents[&1].title, "Second");

        // Same index structure
        assert_eq!(restored.index.len(), original.index.len());

        // Search the restored index and get same results
        let results = restored.search("fast");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Second");
    }

    #[test]
    fn empty_search_index_round_trip() {
        let original = SearchIndex::new();
        let bytes = original.to_bytes().unwrap();
        let restored = SearchIndex::from_bytes(&bytes).unwrap();

        assert_eq!(restored.documents.len(), 0);
        assert_eq!(restored.index.len(), 0);
    }

    #[test]
    fn search_results_match_after_serialization() {
        let mut original = SearchIndex::new();
        original.add_document(
            SearchDocument::new(
                0,
                "Rust Guide".to_string(),
                "/rust/".to_string(),
                "A guide".to_string(),
                vec![],
                vec![],
            ),
            "Rust ownership borrowing lifetimes",
        );
        original.add_document(
            SearchDocument::new(
                1,
                "Python Guide".to_string(),
                "/python/".to_string(),
                "A guide".to_string(),
                vec![],
                vec![],
            ),
            "Python dynamic typing garbage collection",
        );
        original.finalize();

        // Search original
        let original_results = original.search("ownership");

        // Round-trip
        let bytes = original.to_bytes().unwrap();
        let restored = SearchIndex::from_bytes(&bytes).unwrap();
        let restored_results = restored.search("ownership");

        // Same results
        assert_eq!(original_results.len(), restored_results.len());
        assert_eq!(original_results[0].title, restored_results[0].title);
        assert_eq!(original_results[0].path, restored_results[0].path);
    }

    #[test]
    fn search_with_multi_word_query() {
        let mut search_index = SearchIndex::new();
        search_index.add_document(
            SearchDocument::new(
                0,
                "First".to_string(),
                "/first/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "Rust is a systems programming language",
        );
        search_index.add_document(
            SearchDocument::new(
                1,
                "Second".to_string(),
                "/second/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "Rust async programming is fast",
        );
        search_index.finalize();

        let results = search_index.search("rust programming");
        // Both documents match, but each should appear only once
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_results_ordered_by_relevance() {
        let mut search_index = SearchIndex::new();
        search_index.add_document(
            SearchDocument::new(
                0,
                "Common".to_string(),
                "/common/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "rust rust rust programming programming programming",
        );
        search_index.add_document(
            SearchDocument::new(
                1,
                "Unique".to_string(),
                "/unique/".to_string(),
                "summary".to_string(),
                vec![],
                vec![],
            ),
            "rust ownership borrowing lifetimes systems",
        );
        search_index.finalize();

        let results = search_index.search("ownership rust");
        assert_eq!(results.len(), 2);
        // "Unique" should rank first because "ownership" only appears there
        assert_eq!(results[0].title, "Unique");
        assert_eq!(results[1].title, "Common");
    }
}

#[cfg(test)]
mod search_truth_tests {
    use super::*;

    fn doc(id: u32, title: &str) -> SearchDocument {
        SearchDocument::new(
            id,
            title.to_string(),
            format!("/{title}/"),
            "summary".to_string(),
            vec![],
            vec![],
        )
    }

    /// #22: the default search caps results; `search_with_limit` honours a
    /// caller-chosen cap.
    #[test]
    fn search_caps_results_at_the_limit() {
        let mut index = SearchIndex::new();
        for i in 0..15 {
            // Every document matches "rust"; each also has one unique word.
            index.add_document(doc(i, &format!("Doc{i}")), "rust unique_word_{i}");
        }
        index.finalize();

        let results = index.search("rust");
        assert_eq!(results.len(), 10, "default cap is SEARCH_RESULT_LIMIT");
        assert_eq!(results.len(), SEARCH_RESULT_LIMIT);

        let results = index.search_with_limit("rust", 3);
        assert_eq!(results.len(), 3, "explicit cap is honoured");
    }

    /// #22: a term present in every document carries no information
    /// (IDF = ln(1) = 0) — but only on corpora larger than a site where
    /// that is noise rather than the only thing the visitor asked for.
    /// The match is returned at score 0; the cap, not a threshold, keeps
    /// noise out. This test pins that small-site behaviour: a universal
    /// term still matches everything.
    #[test]
    fn search_returns_universal_term_matches() {
        let mut index = SearchIndex::new();
        index.add_document(doc(0, "A"), "shared");
        index.add_document(doc(1, "B"), "shared");
        index.add_document(doc(2, "C"), "shared");
        index.finalize();

        let results = index.search("shared");
        assert_eq!(results.len(), 3, "a universal term matches every document");
    }

    /// #57 regression: sorting must not panic when scores contain NaN
    /// (poisoned via a NaN TF weight — constructible through finalize
    /// arithmetic edge cases); `total_cmp` defines an order for them.
    #[test]
    fn search_with_nan_scores_does_not_panic() {
        let mut index = SearchIndex::new();
        index.add_document(doc(0, "A"), "rust");
        index.add_document(doc(1, "B"), "rust also");
        index.finalize();
        // Inject a NaN score directly — the sort must tolerate it.
        index
            .index
            .entry("rust".to_string())
            .or_default()
            .push((0, f32::NAN));

        let _ = index.search("rust"); // must not panic
    }
}
