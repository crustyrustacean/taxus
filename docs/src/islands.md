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
#[cfg(feature = "islands")]
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
fn hydrate_island(el: Element, component: &str, props_json: &str) {
    match component {
        "Counter" => {
            let props: CounterProps = serde_json::from_str(props_json)
                .unwrap_or(CounterProps { initial: 0 });
            yew::Renderer::<Counter>::with_root_and_props(el.into(), props).hydrate();
        }
        "MyWidget" => {
            let props: MyWidgetProps = serde_json::from_str(props_json)
                .unwrap_or(MyWidgetProps { label: String::new(), count: 0 });
            yew::Renderer::<MyWidget>::with_root_and_props(el.into(), props).hydrate();
        }
        other => tracing::warn!("Unknown island component: {}", other),
    }
}
```

## Initializing a Site with Islands

Use the `--islands` flag when creating a new site:

```bash
cargo run -- init my-site --islands
```

This includes the WASM hydration script in the generated `templates/base.html`:

```html
<script type="module">
  import init from '/wasm/client.js';
  init();
</script>
```

## Building the WASM Client

After building the static site, compile the Yew WASM bundle with Trunk:

```bash
# Build the WASM client
cd client && trunk build --release --dist ../my-site/dist/wasm
```

This writes `client.js` and `client_bg.wasm` into `my-site/dist/wasm/`.

## Development Workflow

### 1. Create a Site with Islands

```bash
cargo run -- init my-site --islands --name "My Site" --base-url "https://example.com"
```

### 2. Build the Static Site

```bash
cargo run --features islands -- build --dir my-site --verbose
```

### 3. Build the WASM Client

```bash
cd client && trunk build --release --dist ../my-site/dist/wasm
```

### 4. Serve and Test

```bash
cd my-site && miniserve dist
# or: python -m http.server 8000 -d dist
```

The page should:
- Render immediately from the pre-rendered HTML
- Show the counter with the initial value
- After WASM loads (~1s), the button becomes interactive

## Feature Flag

The `islands` Cargo feature controls whether Yew SSR is compiled in:

```bash
# Plain SSG — no Yew, fastest compile, smallest binary
cargo run -- build --dir my-site

# Islands SSG — Yew SSR pre-renders components at build time
cargo run --features islands -- build --dir my-site
```

Without the feature:
- `island()` function returns empty string (no error)
- No Yew or WASM dependencies compiled
- Templates that use `{{ island(...) | safe }}` still render without output
