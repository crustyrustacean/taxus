# Getting Started

This guide will help you get up and running with Taxus quickly.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (edition 2024) — [Install Rust](https://www.rust-lang.org/tools/install)

## Quick Start

### Step 1: Clone and Initialize

```bash
# Clone the repository
git clone https://codeberg.org/crustyrustacean/taxus.git
cd taxus

# Create a new site
cargo run -- init my-site --name "My Site" --base-url "https://example.com"
```

This creates the following structure:

```
my-site/
├── site.toml               # Site configuration
├── content/
│   └── _index.md           # Home page
├── templates/
│   ├── base.html           # Base HTML layout
│   ├── page.html           # Single-page template
│   ├── section.html        # Section/listing template
│   ├── tags.html           # Tag listing page
│   ├── tags_term.html      # Individual tag page
│   ├── categories.html     # Category listing page
│   ├── categories_term.html # Individual category page
│   ├── series.html         # Series listing page
│   ├── series_term.html    # Individual series page
│   └── 404.html            # Not found page
├── static/
│   ├── scripts.js          # Placeholder scripts
│   └── favicon.png         # Placeholder favicon
└── styles/
    └── main.scss           # Starter stylesheet
```

### Step 2: Build the Site

```bash
cargo run -- build --dir my-site --verbose
```

This runs the 15-stage build pipeline and writes output to `my-site/dist/`. The WASM client (`client.js` and `client_bg.wasm`) is compiled during the Cargo build, embedded in the binary, and written to `dist/wasm/` automatically — no separate build step is needed.

### Step 3: Serve and View

```bash
cargo run -- serve --dir my-site --open
```

This starts a development server at `http://localhost:3000` and opens it in your browser.

You should see the home page rendered from the Markdown content in `content/_index.md`.

## Next Steps

- Learn about [Configuration](./configuration.md) for customizing your site
- Understand [Content](./content.md) for writing pages and posts
- Explore [Templates](./templates.md) for customizing HTML output
- Read the [CLI Reference](./cli.md) for all command options

## Opting Out of Islands

Islands (Yew/WASM hydration) are enabled by default. If you want a plain
Tera/Markdown scaffold with no WASM hydration, pass `--no-islands` when
initializing:

```bash
cargo run -- init my-site --no-islands
```

See [Islands Architecture](./islands.md) for the complete guide.
