// client/src/main.rs
//
// WASM entry point for the Yew SSG hydration client.
//
// This module is compiled to WebAssembly by Trunk and loaded by the static site.
// On startup it scans for island mount points written by the SSG at build time and
// hydrates each one with the matching Yew component.

// #![no_main] suppresses the implicit Rust binary entry point so that
// #[wasm_bindgen(start)] can be the sole entry without a symbol conflict.
#![no_main]

mod search;

use taxus_common::components::{
    counter::{Counter, CounterProps},
    search_box::{SearchBox, SearchBoxProps},
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

/// helper function to log errors to the console
fn console_error(msg: &str) {
    web_sys::console::error_1(&JsValue::from_str(msg));
}

/// WASM module entry point — called automatically when the module is instantiated.
#[wasm_bindgen(start)]
pub fn hydrate_islands() {
    // Find every island mount point in the document
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
        if let Some(el) = nodes.item(i) {
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
}

fn hydrate_island(name: &str, el: HtmlElement, props_json: &str) {
    match name {
        "Counter" => {
            let props: CounterProps =
                serde_json::from_str(props_json).unwrap_or(CounterProps { initial: 0 });

            // Hydrate: attach Yew to the existing SSR DOM without re-rendering
            yew::Renderer::<Counter>::with_root_and_props(el.into(), props).hydrate();
        }
        "SearchBox" => {
            let props: SearchBoxProps =
                serde_json::from_str(props_json).unwrap_or(SearchBoxProps {
                    placeholder: "".to_string(),
                    max_results: 5,
                });

            yew::Renderer::<SearchBox>::with_root_and_props(el.into(), props).hydrate();
        }
        _ => { /* ignore unknown islands */ }
    }
}
