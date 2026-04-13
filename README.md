# Taxus

[![CI](https://github.com/crustyrustacean/taxus/actions/workflows/ci.yml/badge.svg)](https://github.com/crustyrustacean/taxus/actions/workflows/ci.yml)
[![Security Audit](https://github.com/crustyrustacean/taxus/actions/workflows/security.yml/badge.svg)](https://github.com/crustyrustacean/taxus/actions/workflows/security.yml)
[![Docs](https://github.com/crustyrustacean/taxus/actions/workflows/doc.yml/badge.svg)](https://github.com/crustyrustacean/taxus/actions/workflows/doc.yml)

A Rust-based static site generator built with [Tera](https://keats.github.io/tera/), featuring optional WebAssembly "islands" for interactive components.

## Features

- **Static Site Generation** — Pre-rendered HTML for optimal performance and SEO
- **Markdown + TOML Frontmatter** — Write content with familiar syntax
- **Islands Architecture** — Optional Yew/WASM components that hydrate client-side
- **Full-Text Search** — TF-IDF search index for client-side search (with `islands` feature)
- **Hot-Reloading Dev Server** — WebSocket-based live reload during development
- **RSS/Atom Feeds** — Automatic feed generation
- **Taxonomies** — Tags, categories, and series with automatic archive pages
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
- [trunk](https://trunkrs.dev/) (required only for WASM islands)

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
| `init [dir]` | Create a new site structure |
| `build` | Generate static files |
| `serve` | Start dev server with live reload |
| `clean` | Remove output directory |
| `routes` | List discovered routes |

Common options:
- `--dir <path>` — Site directory (default: current)
- `--verbose` — Debug output
- `--quiet` — Errors only
- `--include-drafts` — Include draft content

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `islands` | off | Enable Yew SSR and WASM hydration |

```bash
# Standard SSG build
cargo run -- build

# With islands support (Yew SSR)
cargo run --features islands -- build
```

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
- [Islands Architecture](docs/src/islands.md)
- [Search](docs/src/search.md)
- [CLI Reference](docs/src/cli.md)
- [API Reference](docs/src/api-reference.md)

Build and serve docs locally:

```bash
cd docs && mdbook serve
```

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `taxus-generator` | SSG library and `taxus` CLI binary |
| `taxus-client` | WASM hydration client (built by Trunk) |
| `taxus-common` | Shared Yew components for SSR and hydration, search index |

## License

MIT — see [License.txt](License.txt).

## Contributing

Pull requests are welcome.

## Acknowledgments

- [Yew](https://yew.rs/) — Rust web framework
- [Tera](https://keats.github.io/tera/) — Template engine
- [Trunk](https://trunkrs.dev/) — WASM bundler
