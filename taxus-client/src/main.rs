//! Taxus WASM hydration client.
//!
//! This crate is compiled to WebAssembly (`wasm32-unknown-unknown`) by the
//! generator's build script (`taxus-generator/build.rs`) using
//! `wasm-bindgen-cli`. The resulting `.js` shim and `.wasm` binary are
//! embedded into the generator binary at compile time and later written to
//! `dist/wasm/` during site builds.
//!
//! # Lifecycle
//!
//! 1. **Build time** — `wasm-bindgen` generates `client.js` (the JS loader)
//!    and `client_bg.wasm` (the compiled WASM module).
//! 2. **Page load** — The SSG-produced HTML includes a `<script>` tag that
//!    loads `client.js`. The loader instantiates the WASM module and calls
//!    [`hydrate_islands()`].
//! 3. **Hydration** — [`hydrate_islands()`] queries the DOM for elements
//!    marked with `data-island`, deserializes their `data-props` JSON, and
//!    attaches Yew component renderers via
//!    [`yew::Renderer::hydrate()`] without re-rendering the SSR output.
//!
//! # Architecture
//!
//! ```text
//!  SSG build                        Browser
//!  ──────────                       ───────
//!  Yew SSR  ──→  <div data-island="SearchBox" data-props='{...}'>
//!                 SSR HTML content                    hydrate_islands()
//!                                                 ──────────────────
//!  client.wasm compiled ──→  loaded by client.js  ──→  mounts Yew event
//!                                                          handlers
//! ```
//!
//! # Modules
//!
//! - [`self`] — WASM entry point; island discovery and hydration dispatch.
//! - [`search`] — Fetches the serialized TF-IDF search index and exposes
//!   a JS-callable `search()` function for the
//!   [`SearchBox`] component.

#![no_main]

mod search;

use taxus_common::components::{
    counter::{Counter, CounterProps},
    search_box::{SearchBox, SearchBoxProps},
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

/// Log an error message to the browser console.
///
/// Used for graceful degradation — hydration failures should never crash the
/// page, so errors are surfaced via `console.error` and the island is simply
/// skipped.
fn console_error(msg: &str) {
    web_sys::console::error_1(&JsValue::from_str(msg));
}

/// WASM entry point — called by the JS shim when the module is instantiated.
///
/// Walks the entire DOM for `[data-island]` elements and hydrates each one
/// with the corresponding Yew component. Islands with unrecognised names are
/// silently ignored so that future components can be added without breaking
/// older cached pages.
///
/// # Contract with the SSG
///
/// Every island mount point must provide two `data-` attributes:
///
/// | Attribute | Type | Description |
/// |-----------|------|-------------|
/// | `data-island` | `str` | Component name (e.g. `"Counter"`, `"SearchBox"`) |
/// | `data-props` | JSON string | Serialized props matching the component's `#[derive(Deserialize)]` struct |
///
/// # Errors
///
/// Failures are logged to the console but never panic. Specific cases:
/// - Missing `window` / `document` → nothing to hydrate.
/// - Failed `querySelectorAll` → DOM not ready.
/// - Unknown island name → skipped.
/// - Invalid `data-props` JSON → component receives default props.
#[wasm_bindgen]
pub fn hydrate_islands() {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return console_error("No window object."),
    };

    let document = match window.document() {
        Some(d) => d,
        None => return console_error("No document object."),
    };

    let nodes = match document.query_selector_all("[data-island]") {
        Ok(n) => n,
        Err(_) => return console_error("Failed to query islands."),
    };

    for i in 0..nodes.length() {
        let Some(el) = nodes.item(i) else {
            continue;
        };

        let el: HtmlElement = match el.dyn_into() {
            Ok(el) => el,
            Err(_) => continue,
        };

        let dataset = el.dataset();
        let name = dataset.get("island").unwrap_or_default();
        let props_json = dataset.get("props").unwrap_or_default();

        hydrate_island(&name, el, &props_json);
    }
}

/// Dispatch hydration to the correct Yew component.
///
/// Each arm deserializes the component's props from JSON. On parse failure
/// the component still mounts but uses its `#[prop_or_default]` values so
/// the page remains functional even with malformed data.
fn hydrate_island(name: &str, el: HtmlElement, props_json: &str) {
    match name {
        "Counter" => {
            let props: CounterProps = serde_json::from_str(props_json).unwrap_or(CounterProps {
                initial: 0,
                class: String::new(),
            });

            // Attach Yew to the existing SSR DOM without re-rendering.
            yew::Renderer::<Counter>::with_root_and_props(el.into(), props).hydrate();
        }
        "SearchBox" => {
            let props: SearchBoxProps =
                serde_json::from_str(props_json).unwrap_or(SearchBoxProps {
                    placeholder: "".to_string(),
                    max_results: 5,
                    class: String::new(),
                });

            yew::Renderer::<SearchBox>::with_root_and_props(el.into(), props).hydrate();
        }
        // Silently skip unknown islands so that adding a new component to
        // the SSG does not break pages that were built with an older client.
        _ => {}
    }
}
