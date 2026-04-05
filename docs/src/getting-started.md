# Getting Started

This guide will help you get up and running with Taxus quickly.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (edition 2024) — [Install Rust](https://www.rust-lang.org/tools/install)
- **trunk** — WebAssembly bundler for Rust — [Install trunk](https://trunkrs.dev/)

## Quick Start

### Step 1: Clone and Initialize

```bash
# Clone the repository
git clone https://github.com/crustyrustacean/taxus.git
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
│   └── section.html        # Section/listing template
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

This runs the 12-stage build pipeline and writes output to `my-site/dist/`.

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

## For Islands Support

If you want interactive Yew components:

```bash
# Initialize with islands support
cargo run -- init my-site --islands

# Build with islands feature enabled
cargo run --features islands -- build --dir my-site

# Build the WASM client
cd client && trunk build --release --dist ../my-site/dist/wasm
```

See [Islands Architecture](./islands.md) for the complete guide.
