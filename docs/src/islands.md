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
<div data-island="Counter" data-props='{&quot;initial&quot;:5}'>
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
{{ island(component="SearchBox", placeholder="Find…", class="docs-search") | safe }}
```

Only the arguments a component's match arm reads are used; any other
keyword is ignored. The props are serialized to JSON and stored in
`data-props`.

| Component | Arguments read by `island()` |
|-----------|------------------------------|
| `Counter` | `initial` (integer, default 0), `class` |
| `SearchBox` | `placeholder` (default `"Search..."`), `class`, `max_results` (default 5, clamped 1–50) |

### The `class` Prop

All island components accept an optional `class` prop that appends custom CSS classes to the component's outer `<div>`:

```html
{{ island(component="SearchBox", class="docs-search") | safe }}
```

This renders as:

```html
<div data-island="SearchBox" data-props='{&quot;placeholder&quot;:&quot;Search...&quot;,&quot;max_results&quot;:5,&quot;class&quot;:&quot;docs-search&quot;}'>
  <!-- component content -->
</div>
```

The serialized props are HTML-entity-escaped inside the single-quoted
attribute (#39), so a prop value containing `'`, `<`, `>`, or `&` cannot
break out of the attribute. The browser's `dataset` accessor un-escapes
the entities when the client reads the attribute, so hydration receives
the original JSON unchanged.

This enables template authors to pass CSS styling hooks for targeting descendant elements without modifying component source.

## Writing an Island Component

Island components live in `taxus-common/src/components/`. Their props must implement `Serialize` and `Deserialize`:

```rust
// taxus-common/src/components/counter.rs
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

Add the component module to `taxus-common/src/components.rs`:

```rust
pub mod counter;
```

## Registering a New Island

Adding an island touches one shared registry plus one explicit arm on
each side; a mismatch is a loud failure, never a silent no-op
([#50](https://github.com/crustyrustacean/taxus/issues/50) collapsed
the free-floating name lists into this design):

1. **The component** lives in `taxus-common/src/components/` with
   `#[derive(Deserialize, Serialize, Properties, PartialEq)]` props,
   and its module is exported from `taxus-common/src/components.rs`.

2. **The registry** — `taxus_common::islands::ISLANDS` — gains one
   entry (`IslandDef { name: "MyWidget" }`, entries kept lexically
   sorted; tests enforce this). Both the generator's SSR dispatch and
   the client's hydration consult this list, so an unregistered name
   never renders at all, and a registered name the client does not
   know is logged and skipped rather than vanishing.

3. **The generator arm**: a match arm in `island()` reading the
   template kwargs into props, plus a `render_island_my_widget`
   helper in `taxus-generator/src/build/pipeline.rs`:

```rust
pub fn render_island_my_widget(props: MyWidgetProps) -> String {
    let props_json = serde_json::to_string(&props).unwrap_or_else(|_| "{}".to_string());
    let ssr_html = block_on_ssr(ServerRenderer::<MyWidget>::with_props(move || props).render());
    island_mount("MyWidget", &props_json, &ssr_html) // escapes data-props (#39)
}
```

4. **The client arm**: a match arm in `taxus-client`'s
   `hydrate_island`. Yew's `Renderer::<T>::hydrate()` needs a concrete
   type per arm, so this step stays explicit — but the registry check
   runs first, so a forgotten arm logs `skipping unknown island:
   MyWidget` in the browser console instead of failing silently:

```rust
fn hydrate_island(name: &str, el: HtmlElement, props_json: &str) {
    if !taxus_common::islands::ISLANDS.iter().any(|i| i.name == name) {
        console_log(&format!("skipping unknown island: {name}"));
        return;
    }
    match name {
        "MyWidget" => {
            let props: MyWidgetProps = serde_json::from_str(props_json)
                .unwrap_or(MyWidgetProps::default());
            yew::Renderer::<MyWidget>::with_root_and_props(el.into(), props).hydrate();
        }
        _ => unreachable!("registry check ran first"),
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
{{ island(component="SearchBox", placeholder="Search...", class="my-search") | safe }}
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
