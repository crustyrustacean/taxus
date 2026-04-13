// common/src/components/search_box.rs

// dependencies
use gloo_timers::callback::Timeout;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

// Props must be serializable so they can be embedded as JSON in the HTML
// and deserialized again at hydration time in the browser.
#[derive(Properties, PartialEq, Clone, Serialize, Deserialize)]
pub struct SearchBoxProps {
    #[prop_or("Search...".to_string())]
    pub placeholder: String,
    #[prop_or(5)]
    pub max_results: usize,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SearchResult {
    pub title: String,
    pub path: String,
    pub summary: String,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "wasmBindings"], catch)]
    async fn search(query: &str) -> Result<JsValue, JsValue>;
}

// A search component with async JS bindings for querying results
#[component]
pub fn SearchBox(props: &SearchBoxProps) -> Html {
    let query = use_state(String::new);
    let results: UseStateHandle<Vec<SearchResult>> = use_state(Vec::new);
    let max_results = props.max_results;
    let timeout_handle: UseStateHandle<Option<Rc<RefCell<Option<Timeout>>>>> = use_state(|| None);

    let on_input = {
        let query = query.clone();
        let results = results.clone();
        let timeout_handle = timeout_handle.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            query.set(value.clone());

            if let Some(handle) = &*timeout_handle {
                handle.borrow_mut().take();
            }

            let results = results.clone();
            let holder = Rc::new(RefCell::new(None));
            let holder_clone = holder.clone();

            let timeout = Timeout::new(200, move || {
                let results = results.clone();
                spawn_local(async move {
                    if value.len() < 2 {
                        results.set(vec![]);
                        return;
                    }
                    match search(&value).await {
                        Ok(js_results) => {
                            if let Ok(parsed) =
                                serde_wasm_bindgen::from_value::<Vec<SearchResult>>(js_results)
                            {
                                let truncated = parsed.into_iter().take(max_results).collect();
                                results.set(truncated);
                            }
                        }
                        Err(_) => results.set(vec![]),
                    }
                });
            });

            *holder_clone.borrow_mut() = Some(timeout);
            timeout_handle.set(Some(holder_clone));
        })
    };

    html! {
        <div class="search-box">
            <input class="search-input" type="text" oninput={on_input} placeholder={props.placeholder.clone()} />
            <ul class="search-results">
                { for (*results).iter().map(|r| html! {
                    <li class="search-result">
                        <a class="search-result-link" href={r.path.clone()}>{ &r.title }</a>
                        <p class="search-result-summary">{ &r.summary }</p>
                    </li>
                })}
            </ul>
        </div>
    }
}
