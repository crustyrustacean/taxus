// common/src/components/counter.rs

// dependencies
use serde::{Deserialize, Serialize};
use yew::prelude::*;

// Props must be serializable so they can be embedded as JSON in the HTML
// and deserialized again at hydration time in the browser.
#[derive(Properties, PartialEq, Clone, Serialize, Deserialize)]
pub struct CounterProps {
    #[prop_or_default]
    pub initial: i32,
}

// A sample component, adds a counter that increments with a button click
#[component]
pub fn Counter(props: &CounterProps) -> Html {
    let count = use_state(|| props.initial);

    let on_click = {
        let count = count.clone();
        Callback::from(move |_| count.set(*count + 1))
    };

    html! {
        <div class="counter">
            <span class="counter-value">{ *count }</span>
            <button class="counter-btn" onclick={on_click}>{ "+" }</button>
        </div>
    }
}
