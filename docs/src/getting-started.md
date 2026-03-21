# Getting Started

This guide will help you get up and running with Yew SSG quickly.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (edition 2024) — [Install Rust](https://www.rust-lang.org/tools/install)
- **trunk** — WebAssembly bundler for Rust — [Install trunk](https://trunkrs.dev/)

## Quick Start

### Step 1 — Initialize a New Site

```bash
# Clone the repository
git clone https://github.com/crustyrustacean/yew-ssg.git
cd yew-ssg

# Create a new site (plain SSG, no islands)
cargo run -- init my-site --name "My Site" --base-url "https://example.com"

# Or create a site with islands support (includes WASM hydration script)
cargo run -- init my-site --name "My Site" --base-url "https://example.com" --islands
```

This creates the following structure in `my-site/`:

```
my-site/
├── site.toml               # Site configuration
├── content/
│   └── _index.md           # Home page (Markdown + TOML frontmatter)
├── templates/
│   ├── base.html           # HTML shell with WASM loader script
│   ├── page.html           # Single-page template (includes Counter island example)
│   └── section.html        # Section/listing template
├── static/
│   ├── scripts.js          # General interactivity (vanilla JS)
│   └── favicon.png         # Placeholder favicon
└── styles/
    └── main.scss           # Starter stylesheet
```

### Step 2 — Build the Static Site

```bash
cargo run -- build --dir my-site --verbose
```

This runs the 6-stage pipeline:
1. Discovers routes from `content/`
2. Loads Tera templates from `templates/`
3. Parses Markdown + frontmatter
4. Renders each page with Tera (including `island()` calls → Yew SSR)
5. Compiles SCSS → CSS, copies static files
6. Writes HTML to `my-site/dist/`

### Step 3 — Build the WASM Client

The interactive Yew components need a compiled WASM bundle:

```bash
cd client && trunk build --release --dist ../my-site/dist/wasm
```

Trunk writes `client.js` and `client_bg.wasm` into `my-site/dist/wasm/`.

> **Important**: The `--dist` path must match the site's own `dist/` directory.
> The `Trunk.toml` default (`../dist/wasm`) is for the workspace-level `dist/`.

### Step 4 — Serve and View

```bash
# Using miniserve
cd my-site && miniserve dist

# Using Python
python -m http.server 8000 -d my-site/dist

# Using npx
npx serve my-site/dist
```

Open `http://localhost:8080` (or the port your server uses).

You should see:
- The home page rendered immediately from SSR HTML
- A Counter showing "3" with a "+" button
- After WASM loads (~1s), the button becomes interactive and increments on click

## Build Commands Reference

| Command | Description |
|---------|-------------|
| `cargo run -- init [path]` | Initialize a new site |
| `cargo run -- init --name "Name" --base-url "https://..."` | Init with custom options |
| `cargo run -- init --islands` | Init with islands support (includes WASM hydration script) |
| `cargo run -- build --dir PATH` | Build a site |
| `cargo run -- build --verbose` | Build with progress output |
| `cargo run -- build --quiet` | Build silently (errors only) |
| `cargo run -- build --include-drafts` | Include draft pages |
| `cargo run -- build --dry-run` | Validate without writing files |
| `cargo run -- build --output PATH` | Override output directory |
| `cargo run -- clean --dir PATH` | Delete generated files |
| `cargo run -- routes --dir PATH` | Inspect discovered routes |
| `cd client && trunk build --release --dist ../SITE/dist/wasm` | Build WASM client |
| `cargo test` | Run all tests |
| `cargo doc` | Generate API documentation |

## Adding an Island Component

To add a new interactive Yew component:

### 1. Write the component in `common`

```rust
// common/src/components/my_widget.rs
use serde::{Deserialize, Serialize};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone, Serialize, Deserialize)]
pub struct MyWidgetProps {
    pub label: String,
}

#[function_component(MyWidget)]
pub fn my_widget(props: &MyWidgetProps) -> Html {
    html! { <div class="widget">{ &props.label }</div> }
}
```

### 2. Register SSR in the generator

In `generator/src/templates/renderer.rs`, add a match arm to the `island()` function closure:

```rust
"MyWidget" => {
    use common::components::my_widget::{MyWidget, MyWidgetProps};
    use crate::build::pipeline::render_island_generic;
    let label = args.get("label")
        .and_then(Value::as_str)
        .unwrap_or("").to_string();
    render_island_generic::<MyWidget>(MyWidgetProps { label }, "MyWidget")
}
```

### 3. Register hydration in the client

In `client/src/main.rs`, add a match arm to `hydrate_island()`:

```rust
"MyWidget" => {
    let props: MyWidgetProps = serde_json::from_str(props_json)
        .unwrap_or(MyWidgetProps { label: String::new() });
    yew::Renderer::<MyWidget>::with_root_and_props(el.into(), props).hydrate();
}
```

### 4. Use it in a template

```html
{{ island(component="MyWidget", label="Hello from Yew!") | safe }}
```

## Next Steps

- Learn about [Configuration](./configuration.md)
- Understand the [Content system](./content.md)
- Explore [Templates](./templates.md)
- Read the [API Reference](./api-reference.md)
