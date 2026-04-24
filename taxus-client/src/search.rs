//! Client-side search backed by a TF-IDF index.
//!
//! This module exposes a single JS-callable async function [`search()`] that
//! the [`SearchBox`](taxus_common::components::search_box::SearchBox) Yew
//! component invokes via `window.wasmBindings.search()`.
//!
//! # Index lifecycle
//!
//! 1. The SSG build writes a postcard-serialized [`SearchIndex`] to
//!    `dist/search_index.bin`.
//! 2. On first search, [`load_search_index()`] fetches the binary via
//!    `fetch("/search_index.bin")` and deserialises it.
//! 3. The deserialized index is stored in a [`thread_local!`]
//!    [`OnceCell`] so subsequent queries reuse the same in-memory copy.
//!
//! # JS interface
//!
//! ```text
//! // Called by the SearchBox component through wasm-bindgen.
//! let results = await window.wasmBindings.search("rust");
//! // results: Array<{ id, title, path, summary, tags, categories }>
//! ```
//!
//! Each element in the returned array is a [`SearchDocument`] serialised to
//! a JS object via `serde-wasm-bindgen`.

use js_sys::Uint8Array;
use std::cell::OnceCell;
use taxus_common::search::{SearchDocument, SearchIndex};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, Response};

// Lazily-loaded search index, cached for the lifetime of the WASM module.
//
// Wrapped in `OnceCell` so the index is fetched at most once — the first
// call to `search()` triggers the load, and every subsequent call reuses
// the cached copy without additional network requests.
thread_local! {
    static SEARCH_INDEX: OnceCell<SearchIndex> = const { OnceCell::new() };
}

/// Fetch the pre-built search index from the server and deserialize it.
///
/// Expects a `postcard`-encoded binary at `/search_index.bin` (written to
/// `dist/search_index.bin` by the SSG build pipeline).
///
/// # Errors
///
/// Returns a [`JsValue`] error if:
/// - `window` is unavailable (not running in a browser context).
/// - The `fetch` request fails (network error, 404, etc.).
/// - The response body cannot be decoded as a valid [`SearchIndex`].
async fn load_search_index() -> Result<SearchIndex, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;

    let request = Request::new_with_str("/search_index.bin")?;

    let response_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: Response = response_value.dyn_into()?;

    let buffer_value = JsFuture::from(response.array_buffer()?).await?;
    let uint8_array = Uint8Array::new(&buffer_value);
    let bytes = uint8_array.to_vec();

    let index = SearchIndex::from_bytes(&bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(index)
}

/// Perform a full-text search against the cached site index.
///
/// This is the function exported to JavaScript via `wasm-bindgen`. The
/// [`SearchBox`](taxus_common::components::search_box::SearchBox) component calls it through `window.wasmBindings.search()`.
///
/// # Behaviour
///
/// - On the first call the binary index is fetched from `/search_index.bin`,
///   deserialized, and cached in thread-local storage.
/// - The query is tokenized, stemmed, and scored with TF-IDF by
///   [`SearchIndex::search()`].
/// - Matching [`SearchDocument`]s are serialized to JS objects and returned
///   as a `JsValue` (JavaScript `Array`).
///
/// # Arguments
///
/// * `query` — Free-text search string. Queries shorter than 2 characters
///   may produce no results depending on the tokenizer's minimum-token
///   length filter.
///
/// # Returns
///
/// A `JsValue` resolving to a JavaScript `Array` of objects:
///
/// ```text
/// [
///   { id: 0, title: "...", path: "/blog/...", summary: "...",
///     tags: ["rust"], categories: ["programming"] },
///   ...
/// ]
/// ```
///
/// # Errors
///
/// Returns a rejected `Promise` if the index cannot be fetched or parsed.
#[wasm_bindgen]
pub async fn search(query: &str) -> Result<JsValue, JsValue> {
    // Lazily load the index on first call.
    let is_loaded = SEARCH_INDEX.with(|cell| cell.get().is_some());

    if !is_loaded {
        let index = load_search_index().await?;
        SEARCH_INDEX.with(|cell| {
            let _ = cell.set(index);
        });
    }

    // Run the query and serialize results to JS.
    let result = SEARCH_INDEX.with(|cell| {
        let index = cell
            .get()
            .expect("index should be loaded after the block above");

        let documents: Vec<&SearchDocument> = index.search(query);
        let array = js_sys::Array::new();

        for doc in documents {
            if let Ok(val) = serde_wasm_bindgen::to_value(doc) {
                array.push(&val);
            }
        }

        array
    });

    Ok(result.into())
}
