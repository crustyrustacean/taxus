# Search

Taxus provides a full-text search index that can be shipped to the browser for client-side search. The index uses TF-IDF (Term Frequency-Inverse Document Frequency) ranking with English stemming.

## Overview

When the `islands` feature is enabled, the build pipeline generates a search index at `dist/search_index.bin`. This binary file contains:

- **Document metadata** — Title, path, summary, tags, and categories for each page
- **Inverted index** — Mapping from word stems to document IDs with TF-IDF scores

The index is serialized with `postcard` for compact storage and fast deserialization in the browser.

## Enabling Search

Search requires the `islands` feature:

```bash
cargo run --features islands -- build --dir my-site
```

This generates `dist/search_index.bin` alongside your static files.

## How It Works

### Indexing Pipeline

1. **Tokenization** — Content is split into lowercase words, filtering out words shorter than 3 characters
2. **Stemming** — Words are reduced to their root form using the Porter stemmer (e.g., "programming" → "program")
3. **TF-IDF Scoring** — Each term gets a weight based on:
   - **Term Frequency (TF)** — How often the term appears in a document
   - **Inverse Document Frequency (IDF)** — How rare the term is across all documents

### Search Query Processing

When a user searches:

1. The query is tokenized and stemmed using the same process
2. Each stem's postings are retrieved from the index
3. TF-IDF scores are summed for matching documents
4. Results are returned sorted by relevance score

## Output Format

The search index is written to `dist/search_index.bin` in `postcard` binary format. To use it client-side:

```rust
// In WASM client
let bytes = fetch("/search_index.bin").await;
let index = SearchIndex::from_bytes(&bytes);
let results = index.search("rust programming");
```

Each `SearchDocument` in the results contains:

| Field | Description |
|-------|-------------|
| `id` | Unique document identifier |
| `title` | Page title from frontmatter |
| `path` | URL path (e.g., `/blog/my-post/`) |
| `summary` | Page summary for display |
| `tags` | Tags from frontmatter |
| `categories` | Categories from frontmatter |

## Using Search Client-Side

### Fetch and Deserialize

```javascript
// Fetch the binary index
const response = await fetch('/search_index.bin');
const buffer = await response.arrayBuffer();
const bytes = new Uint8Array(buffer);

// Use postcard (or a Rust WASM module) to deserialize
// The SearchIndex struct is defined in taxus-common
```

### Integration with Yew

Create a search island component that loads the index on mount:

```rust
// In taxus-common/src/components/search.rs
use yew::prelude::*;
use crate::search::{SearchIndex, SearchDocument};

#[function_component(SearchBox)]
pub fn search_box() -> Html {
    let index = use_state(|| None::<SearchIndex>);
    let query = use_state(|| String::new());
    let results = use_state(|| Vec::<SearchDocument>::new);

    // Load index on mount
    {
        let index = index.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let resp = gloo::net::http::Request::get("/search_index.bin")
                    .send()
                    .await
                    .unwrap();
                let bytes = resp.binary().await.unwrap();
                index.set(Some(SearchIndex::from_bytes(&bytes)));
            });
            || ()
        });
    }

    // Render search UI
    html! {
        <div class="search">
            <input
                type="text"
                placeholder="Search..."
                oninput={|e| query.set(e.value())}
            />
            <ul class="search-results">
                { for results.iter().map(|doc| html! {
                    <li>
                        <a href={doc.path.clone()}>{ &doc.title }</a>
                        <p>{ &doc.summary }</p>
                    </li>
                })}
            </ul>
        </div>
    }
}
```

## API Reference

### `SearchDocument`

```rust
pub struct SearchDocument {
    pub id: u32,
    pub title: String,
    pub path: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}
```

### `SearchIndex`

```rust
pub struct SearchIndex {
    pub documents: Vec<SearchDocument>,
    pub index: HashMap<String, Vec<(u32, f32)>>,
}
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create an empty index |
| `add_document(doc, content)` | Add a document with its content |
| `search(query) -> Vec<&SearchDocument>` | Search and return ranked results |
| `finalize()` | Apply IDF weighting (call after all documents added) |
| `to_bytes() -> Vec<u8>` | Serialize to binary |
| `from_bytes(bytes) -> Self` | Deserialize from binary |

### Helper Functions

```rust
pub fn tokenize(text: &str) -> Vec<String>
```

Splits text into lowercase tokens, filtering words shorter than 3 characters.

```rust
pub fn stem(tokens: &[String]) -> Vec<String>
```

Applies English Porter stemmer to tokens.

## Performance

- **Index size** — Typically 10-30% of total content size
- **Deserialization** — Near-instant with postcard format
- **Search latency** — Sub-millisecond for typical queries

## Limitations

- **English only** — Stemming is currently English-only
- **No phrase search** — Queries are treated as bag-of-words
- **No highlighting** — Results don't include matched snippets
