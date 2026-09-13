# Search

Taxus provides a built-in search component with client-side full-text search. The `SearchBox` island component uses TF-IDF (Term Frequency-Inverse Document Frequency) ranking with English stemming.

## Overview

The build pipeline:

1. Generates a search index at `dist/search_index.bin` (stage 13, from every
   processed page in tree order; see [Architecture](./architecture.md))
2. The `SearchBox` component is available for use in templates

The indexed text for each page is its **Markdown body** — not the rendered
HTML, whose tags, attributes and highlighter classes would pollute the term
space — plus its **title** and **tags/categories**, each repeated as a field
boost so a query matching only the title finds the page.

The binary index contains:

- **Document metadata** — Title, path, summary, tags, and categories for each page
- **Inverted index** — Mapping from word stems to document IDs with TF-IDF scores

The index is serialized with `postcard` for compact storage and fast deserialization in the browser.

## Enabling Search

Search is on by default; the index is generated automatically:

```bash
cargo run -- build --dir my-site
```

This generates `dist/search_index.bin` alongside your static files. A site
with no search box can skip it — set `search = false` in `[build]`:

```toml
[build]
search = false
```

## Using the SearchBox Component

The `SearchBox` island component provides a ready-to-use search interface. Add it to any template:

```html
<div class="search-container">
  {{ island(component="SearchBox") | safe }}
</div>
```

### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `placeholder` | string | `"Search..."` | Placeholder text for the input |
| `class` | string | `""` | Custom CSS classes to append to the outer container |

The component also accepts a `max_results` prop (integer, default 5,
clamped to 1–50): the number of results shown in the dropdown.

Example with custom props:

```html
{{ island(component="SearchBox", placeholder="Find content...", class="docs-search") | safe }}
```

### Styling

The component uses these CSS classes that you can style:

| Class | Element |
|-------|---------|
| `.search-box` | Container div |
| `.search-input` | Text input field |
| `.search-results` | Results list (`<ul>`) |
| `.search-result` | Individual result item (`<li>`) |
| `.search-result-link` | Result title link |
| `.search-result-summary` | Result summary text |

Use the `class` prop to add custom classes for styling hooks:

```html
{{ island(component="SearchBox", class="docs-search") | safe }}
```

Then target the custom class in your SCSS:

```scss
.docs-search .search-input {
  // Custom styles for docs search input
}
```

Example SCSS:

```scss
.search-container {
  max-inline-size: 48rem;
  margin-inline: auto;
  padding-inline: 1.5rem;
}

.search-input {
  font-family: var(--font-mono);
  font-size: 0.95rem;
  padding: 0.6rem 1rem;
  border-radius: 0.5rem;
  border: 1px solid var(--border);
  background-color: var(--bg-surface);
  color: var(--text);
}

.search-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.search-result {
  background-color: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  padding: 0.75rem 1rem;
}

.search-result-link {
  font-family: var(--font-mono);
  font-weight: 600;
  color: var(--accent);
  text-decoration: none;
}

.search-result-summary {
  font-size: 0.85rem;
  color: var(--text-muted);
}
```

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
4. Results are returned sorted by relevance score, capped at 10 — a cap,
   not a score threshold, because TF-IDF has no corpus-independent floor:
   on a three-page site every match is a good match

### Component Architecture

The `SearchBox` component:

1. Uses a 200ms debounce on input to avoid excessive queries
2. Requires at least 2 characters before searching
3. Calls the `window.wasmBindings.search()` function exposed by the WASM client
4. The WASM client lazily loads the search index on first use
5. Results are truncated to `max_results` and displayed in a list

## Output Format

The search index is written to `dist/search_index.bin` in `postcard` binary format.

Each `SearchDocument` in the results contains:

| Field | Description |
|-------|-------------|
| `id` | Unique document identifier |
| `title` | Page title from frontmatter |
| `path` | URL path (e.g., `/blog/my-post/`) |
| `summary` | Page summary for display |
| `tags` | Tags from frontmatter |
| `categories` | Categories from frontmatter |

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
    pub documents: BTreeMap<u32, SearchDocument>,
    pub index: HashMap<String, Vec<(u32, f32)>>,
}
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create an empty index |
| `add_document(doc, content)` | Add a document with its content |
| `search(query) -> Vec<&SearchDocument>` | Search and return ranked results |
| `finalize()` | Apply IDF weighting (call after all documents added) |
| `to_bytes() -> Result<Vec<u8>, postcard::Error>` | Serialize to binary |
| `from_bytes(bytes) -> Result<Self, postcard::Error>` | Deserialize from binary |

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
- **Lazy loading** — Index is loaded only when first search is performed

## Limitations

- **English only** — Stemming is currently English-only
- **No phrase search** — Queries are treated as bag-of-words
- **No highlighting** — Results don't include matched snippets
