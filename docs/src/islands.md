# Islands Architecture

Taxus implements the **Islands Architecture**: Tera templates render the static "sea" of HTML, while Yew components are pre-rendered server-side into "islands" and then hydrated by the WASM client in the browser.

## How It Works

```
BUILD TIME (generator)                BROWSER
─────────────────────                 ───────
1. Tera renders page shell            4. HTML is immediately visible (no JS needed)
2. island() calls Yew SSR             5. WASM loads asynchronously
3. SSR HTML + props JSON emitted      6. Yew hydrates mount points → interactive
```

### Build-Time: Server-Side Rendering

When you use `{{ island(component="Counter", initial=5) | safe }}` in a template:

1. The `island()` Tera function is called during template rendering
2. Yew SSR renders the component to HTML
3. The output is wrapped in a mount point div with serialized props:

```html
<div data-island="Counter" data-props='{"initial":5}'>
  <!-- Pre-rendered by Yew SSR: -->
  <div class="counter"><span>5</span><button>+</button></div>
</div>
```

### Browser-Time: Hydration

When the page loads:

1. The pre-rendered HTML is immediately visible (no JavaScript required)
2. The WASM bundle loads asynchronously
3. The client finds all `[data-island]` elements in the DOM
4. For each island: deserialize `data-props`, call `yew::Renderer::hydrate()`
5. The component becomes interactive without re-rendering

## Two-Tier Interactivity

taxus supports two layers of interactivity:

| Tier | Technology | Use For |
|------|------------|---------|
| 1 — General | `static/scripts.js` (vanilla JS) | DOM manipulation, toggles, analytics, lightweight events |
| 2 — Performance | Yew WASM island | Heavy computation, complex reactive state, data-intensive UI |

Use vanilla JS for simple interactions; reserve Yew islands for components that benefit from the Yew component model.

## Using Islands in Templates

### The `island()` Tera Function

Use the `island()` function in any `.html` template:

```html
{% extends "base.html" %}

{% block content %}
  {{ page.content | safe }}
  
  <h2>Interactive Counter</h2>
  {{ island(component="Counter", initial=5) | safe }}
{% endblock %}
```

**Important**: Always use `| safe` after `island()` to prevent HTML escaping.

### Passing Props

Props are passed as keyword arguments to `island()`:

```html
{{ island(component="Counter", initial=5) | safe }}
{{ island(component="MyWidget", label="Hello", count=3) | safe }}
```

The props are serialized to JSON and stored in `data-props`.

### The `class` Prop

All island components accept an optional `class` prop that appends custom CSS classes to the component's outer `<div>`:

```html
{{ island(component="SearchBox", class="docs-search") | safe }}
```

This renders as:

```html
<div data-island="SearchBox" data-props='{"placeholder":"Search...","max_results":5,"class":"docs-search"}' class="search-box docs-search">
  <!-- component content -->
</div>
```

This enables template authors to pass CSS styling hooks for targeting descendant elements without modifying component source.

## Writing an Island Component

Island components live in `common/src/components/`. Their props must implement `Serialize` and `Deserialize`:

```rust
// common/src/components/counter.rs
use serde::{Deserialize, Serialize};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone, Serialize, Deserialize)]
pub struct CounterProps {
    #[prop_or_default]
    pub initial: i32,
    #[prop_or_default]
    pub class: String,
}

#[function_component(Counter)]
pub fn counter(props: &CounterProps) -> Html {
    let count = use_state(|| props.initial);
    let on_click = {
        let count = count.clone();
        Callback::from(move |_| count.set(*count + 1))
    };
    
    html! {
        <div class="counter">
            <span>{ *count }</span>
            <button onclick={on_click}>{ "+" }</button>
        </div>
    }
}
```

### Export the Component

Add the component module to `common/src/components/mod.rs`:

```rust
pub mod counter;
```

## Registering a New Island

Two registries must be updated in sync:

### 1. Generator SSR Registry

In `generator/src/templates/renderer.rs`, add a match arm to the `island()` Tera function:

```rust
tera.register_function("island", |args: &HashMap<String, tera::Value>| {
    use tera::Value;
    
    let component = args.get("component").and_then(Value::as_str).unwrap_or("");
    
    let html = match component {
        "Counter" => {
            use crate::build::pipeline::render_island_counter;
            use common::components::counter::CounterProps;
            
            let initial = args.get("initial").and_then(Value::as_i64).unwrap_or(0) as i32;
            
            render_island_counter(CounterProps { initial })
        }
        "SearchBox" => {
            use crate::build::pipeline::render_search_box;
            use common::components::search_box::SearchBoxProps;
            
            let placeholder = args.get("placeholder")
                .and_then(Value::as_str)
                .unwrap_or("Search...")
                .to_string();
            let max_results = args.get("max_results")
                .and_then(Value::as_i64)
                .unwrap_or(5) as usize;
            
            render_search_box(SearchBoxProps { placeholder, max_results })
        }
        "MyWidget" => {
            // Add your component here
            use crate::build::pipeline::render_island_generic;
            use common::components::my_widget::{MyWidget, MyWidgetProps};
            
            let label = args.get("label")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let count = args.get("count")
                .and_then(Value::as_i64)
                .unwrap_or(0) as i32;
            
            render_island_generic::<MyWidget>(
                MyWidgetProps { label, count },
                "MyWidget"
            )
        }
        other => format!("<!-- unknown island: {other} -->"),
    };
    
    Ok(Value::String(html))
});
```

### 2. Client Hydration Registry

In `client/src/main.rs`, add a match arm to the hydration function:

```rust
fn hydrate_island(name: &str, el: HtmlElement, props_json: &str) {
    match name {
        "Counter" => {
            let props: CounterProps = serde_json::from_str(props_json)
                .unwrap_or(CounterProps { initial: 0 });
            yew::Renderer::<Counter>::with_root_and_props(el.into(), props).hydrate();
        }
        "SearchBox" => {
            let props: SearchBoxProps = serde_json::from_str(props_json)
                .unwrap_or(SearchBoxProps {
                    placeholder: String::new(),
                    max_results: 5,
                });
            yew::Renderer::<SearchBox>::with_root_and_props(el.into(), props).hydrate();
        }
        "MyWidget" => {
            let props: MyWidgetProps = serde_json::from_str(props_json)
                .unwrap_or(MyWidgetProps { label: String::new(), count: 0 });
            yew::Renderer::<MyWidget>::with_root_and_props(el.into(), props).hydrate();
        }
        _ => { /* ignore unknown islands */ }
    }
}
```

## Built-in Islands

Taxus includes two built-in island components:

### Counter

A simple counter with increment button. This is an example component demonstrating the islands architecture — useful for testing and learning, but not intended for production use.

### SearchBox

A production-ready search component with debounced input and async results. See [Search](./search.md) for full documentation.

```html
{{ island(component="SearchBox", placeholder="Search...", max_results=10, class="my-search") | safe }}
```

## Initializing a Site with Islands

Islands are enabled by default. Initializing a new site includes the WASM
hydration script in the generated `templates/base.html` and a `Counter`
island demo in `templates/section.html`:

```bash
cargo run -- init my-site
```

The generated `base.html` includes the WASM hydration script:

```html
<script type="module">
  import init, * as bindings from '/wasm/client.js';
  const wasm = await init({ module_or_path: '/wasm/client_bg.wasm' });
  window.wasmBindings = bindings;
  bindings.hydrate_islands();
</script>
```

To generate a plain Tera/Markdown scaffold with no WASM hydration, pass
`--no-islands`:

```bash
cargo run -- init my-site --no-islands
```

## Building the WASM Client

The WASM client is compiled automatically as part of every Cargo build. A Cargo build script (`taxus-generator/build.rs`) compiles the `taxus-client` crate to `wasm32-unknown-unknown`, runs `wasm-bindgen` to generate JS bindings, and embeds the resulting `client.js` and `client_bg.wasm` into the generator binary via `include_bytes!`. At site build time, these embedded files are written to `dist/wasm/`.

No separate build step or external tooling (such as Trunk) is required.

## Development Workflow

### 1. Create a Site

```bash
cargo run -- init my-site --name "My Site" --base-url "https://example.com"
```

### 2. Build the Static Site (includes WASM client)

```bash
cargo run -- build --dir my-site --verbose
```

The WASM client is compiled as part of the Cargo build and embedded in the binary. During the `taxus build` pipeline, the embedded `client.js` and `client_bg.wasm` are written to `dist/wasm/` automatically — no separate build step needed.

### 3. Serve and Test

```bash
cargo run -- serve --dir my-site --open
```

The page should:
- Render immediately from the pre-rendered HTML
- Show the counter with the initial value
- After WASM loads (~1s), the button becomes interactive

## Search

The build pipeline also generates a search index. See [Search](./search.md) for details.
