# Taxus

A Rust-based static site generator built with [Tera](https://keats.github.io/tera/), featuring WebAssembly "islands" for interactive components.

## Features

- **Static Site Generation** — Pre-rendered HTML for optimal performance and SEO
- **Markdown + TOML Frontmatter** — Write content with familiar syntax
- **Islands Architecture** — Yew/WASM components that hydrate client-side
- **Syntax Highlighting** — Tree-sitter based code highlighting (Rust built in)
- **Full-Text Search** — TF-IDF search index for client-side search
- **Hot-Reloading Dev Server** — WebSocket-based live reload during development
- **RSS/Atom Feeds** — Automatic feed generation
- **Taxonomies** — Tags, categories, and series with automatic archive pages (list + term templates included in scaffold)
- **Co-located Assets** — Images in content directories copy to output
- **Hero Images** — Responsive hero images with automatic WebP conversion and srcset generation

## Installation

```bash
git clone https://github.com/crustyrustacean/taxus.git
cd taxus
cargo build --release
```

Prerequisites:
- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)

## Quick Start

```bash
# Create a new site
cargo run -- init my-site

# Build the site
cargo run -- build --dir my-site

# Start development server with hot reload
cargo run -- serve --dir my-site
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `init [PATH]` | Scaffold a new site structure |
| `build` | Generate static files |
| `serve` | Start dev server with live reload |
| `clean` | Remove output directory |
| `routes` | List discovered routes |

### Notable options

**`init`**

- `-n, --name <NAME>` — Site name
- `-u, --base-url <URL>` — Base URL
- `-f, --force` — Initialize even if directory is not empty
- `--no-islands` — Disable WASM islands hydration (enabled by default)

**`build`**

- `--include-drafts` — Include draft content
- `--dry-run` — Simulate without writing files
- `--clean` — Remove output directory before building
- `-o, --output <PATH>` — Override the output directory

**`serve`**

- `-p, --port <PORT>` — Port to listen on (default: 3000)
- `-o, --open` — Open browser automatically

**Common**

- `-d, --dir <path>` — Site directory (default: current)
- `-v, --verbose` — Debug output
- `-q, --quiet` — Errors only

The workspace also ships an `xtask` task runner (`cargo xtask`) wrapping
build, test, lint, doc, and release workflows — see
[Development](docs/src/development.md).

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `lang-rust` | on | Rust syntax highlighting via tree-sitter |

Islands (Yew SSR + WASM hydration) are always enabled — they are a first-class part of the generator. No feature flag is required.

## Hero Images

Add a hero image to any page with two lines of frontmatter:

```toml
+++
title = "My Post"
hero_image = "sunset.jpg"
hero_alt = "A mountain sunset"
+++
```

Taxus automatically generates responsive variants (400/800/1200px), converts to WebP, and produces a `<picture>` element with srcset. Configure breakpoints and format in `site.toml`:

```toml
[images]
widths = [400, 800, 1200]
format = "webp"
```

## Project Structure

```
my-site/
├── site.toml      # Site configuration
├── content/       # Markdown pages
├── templates/     # Tera HTML templates
├── static/        # Static assets
└── styles/        # SCSS stylesheets
```

## Configuration

`site.toml`:

```toml
[site]
name = "My Site"
base_url = "https://example.com"

[build]
output_dir = "dist"
```

## Documentation

Comprehensive documentation is available in the `docs/` directory:

- [Introduction](docs/src/introduction.md)
- [Getting Started](docs/src/getting-started.md)
- [Architecture](docs/src/architecture.md)
- [Configuration](docs/src/configuration.md)
- [Content](docs/src/content.md)
- [Images](docs/src/images.md)
- [Templates](docs/src/templates.md)
- [Syntax Highlighting](docs/src/syntax-highlighting.md)
- [Islands Architecture](docs/src/islands.md)
- [Search](docs/src/search.md)
- [Styling](docs/src/styling.md)
- [CLI Reference](docs/src/cli.md)
- [Development Server](docs/src/serve.md)
- [Development](docs/src/development.md)
- [API Reference](docs/src/api-reference.md)

Build and serve docs locally:

```bash
cd docs && mdbook serve
```

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `taxus-generator` | SSG library and `taxus` CLI binary |
| `taxus-client` | WASM hydration client (built into the generator binary at compile time) |
| `taxus-common` | Shared Yew components for SSR and hydration, search index |
| `xtask` | Workspace task runner (`cargo xtask`) for build, test, lint, release, … |

## License

MIT — see [License.txt](License.txt).

## Contributing

Pull requests are welcome.

## Acknowledgments

- [Yew](https://yew.rs/) — Rust web framework
- [Tera](https://keats.github.io/tera/) — Template engine
- [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) — WASM/JS interop (invoked automatically at build time)