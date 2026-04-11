// taxus-client/src/search.rs

use js_sys::Uint8Array;
use std::cell::OnceCell;
use taxus_common::search::SearchIndex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, Response};

thread_local! {
    static SEARCH_INDEX: OnceCell<SearchIndex> = OnceCell::new();
}

pub async fn load_search_index() -> Result<SearchIndex, JsValue> {
    let window = web_sys::window().ok_or(JsValue::from_str("no window"))?;

    let request = Request::new_with_str("/search_index.bin")?;

    let response_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: Response = response_value.dyn_into()?;

    let buffer_value = JsFuture::from(response.array_buffer()?).await?;
    let uint8_array = Uint8Array::new(&buffer_value);
    let bytes = uint8_array.to_vec();

    let index = SearchIndex::from_bytes(&bytes);

    Ok(index)
}

#[wasm_bindgen]
pub async fn search(query: &str) -> Result<JsValue, JsValue> {
    // Load index if not already loaded
    let is_loaded = SEARCH_INDEX.with(|cell| cell.get().is_some());

    if !is_loaded {
        let index = load_search_index().await?;
        SEARCH_INDEX.with(|cell| {
            let _ = cell.set(index);
        });
    }

    // Run the search
    let result = SEARCH_INDEX.with(|cell| {
        let index = cell.get().expect("index should be loaded");
        let results = index.search(query);
        let array = js_sys::Array::new();
        for doc in results {
            if let Ok(val) = serde_wasm_bindgen::to_value(doc) {
                array.push(&val);
            }
        }
        array
    });

    Ok(result.into())
}
