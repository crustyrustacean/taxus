# Taxus

A Rust-based static site generator built with [Tera](https://keats.github.io/tera/), featuring WebAssembly "islands" for interactive components.

Taxus is a compiler for websites: a folder of Markdown, templates, styles
and static files goes in, a folder of HTML and a few generated files comes
out. The build parses the content directory into a Site Tree, computes
every listing, feed and index from that tree, and writes the result. The
[Theory](docs/src/theory/overview.md) chapters explain the model.

## Features

- **Static site generation**: pre-rendered HTML, no JavaScript needed to read a page
- **Markdown with TOML frontmatter**: one file per page, `_index.md` per section
- **Site Tree**: sections and pages in memory; URLs, listings, feeds and the sitemap are derived from it
- **Islands**: Yew components rendered to HTML at build time and hydrated by WASM in the browser
- **Syntax highlighting**: tree-sitter, Rust grammar built in
- **Full-text search**: a TF-IDF index searched in the browser by the `SearchBox` island
- **Development server**: rebuilds on change and reloads the browser over a WebSocket
- **RSS and Atom feeds**: dated pages, newest first
- **Taxonomies**: tags, categories and series, with list and term pages
- **Pagination**: `paginate_by` on any section
- **Co-located assets**: non-Markdown files in `content/` are copied to the output
- **Hero images**: responsive variants, WebP conversion, `<picture>` markup
- **Internal links**: `@/path.md` links checked at build time
- **Sitemap, robots.txt, 404 page, alias redirects**

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

The table is the `Commands:` block of `taxus --help`.

| Command | Description |
|---------|-------------|
| `build` | Build the static site from Markdown content and templates |
| `clean` | Remove all generated files from the output directory |
| `init` | Initialize a new site with a default directory structure |
| `routes` | List all routes that would be discovered from the content directory |
| `serve` | Start a development server with live reload |
| `help` | Print this message or the help of the given subcommand(s) |

### Notable options

**`init [PATH]`**

- `-n, --name <NAME>` — Site name used in templates and site.toml
- `-u, --base-url <URL>` — Base URL for the site (must start with http:// or https://)
- `-f, --force` — Initialize even if the directory is not empty
- `--no-islands` — Disable islands support for a plain Tera/Markdown scaffold

**`build`**

- `--include-drafts` — Include pages marked `draft = true` in frontmatter
- `--dry-run` — Simulate the build without writing any output files
- `--clean` — Remove all files from the output directory before building
- `-o, --output <PATH>` — Override the output directory from site.toml

**`serve`**

- `--host <ADDR>` — IP address to listen on (default: 127.0.0.1)
- `-p, --port <PORT>` — Port to listen on (default: 3000)
- `-o, --open` — Open the site in a browser after starting the server

The dev server binds to loopback only, so it is reachable from your machine
and nothing else. To test on another device on your network (a phone, say),
opt in explicitly with `taxus serve --host 0.0.0.0` and open
`http://<your-lan-ip>:3000` on that device.

**Common**

- `-d, --dir <PATH>` — Root directory of the site (must contain site.toml); default `.`
- `-v, --verbose` — Print detailed progress for each build stage (`build`, `serve`)
- `-q, --quiet` — Suppress all output except errors (`build`, `serve`)

The workspace also ships an `xtask` task runner (`cargo xtask`) wrapping
build, test, lint, doc, and release workflows — see
[Development](docs/src/development.md).

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `lang-rust` | on | Rust syntax highlighting via tree-sitter |
| `webp-lossy` | on | Lossy WebP hero image variants via libwebp; without it WebP output is lossless and `images.quality` is ignored for WebP |

Islands (Yew SSR + WASM hydration) are always compiled in. No feature flag is required; `taxus init --no-islands` only leaves the hydration script out of the scaffold.

## Hero Images

Add a hero image to any page with two lines of frontmatter:

```toml
+++
title = "My Post"
hero_image = "sunset.jpg"
hero_alt = "A mountain sunset"
+++
```

Taxus automatically generates responsive variants (400/800/1200px), converts to lossy WebP, and produces a `<picture>` element with srcset. Configure breakpoints, format and quality in `site.toml`:

```toml
[images]
widths = [400, 800, 1200]
format = "webp"   # "webp", "jpeg" (or "jpg"), or "png"
quality = 80      # 1-100; applies to jpeg and webp only, png ignores it
```

Lossy WebP is encoded with libwebp via the `webp-lossy` cargo feature, which is on by default. Building with `--no-default-features` drops the C dependency: WebP output is then lossless (via the `image` crate) and `quality` is ignored for WebP, with a warning at build time. Processed variants are cached by content hash and quality, so changing `quality` re-encodes on the next build.

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
- [Theory](docs/src/theory/overview.md): [Site Tree](docs/src/theory/site-tree.md), [Identity](docs/src/theory/identity.md), [Derivations](docs/src/theory/derivations.md), [Worked Example](docs/src/theory/worked-example.md), [Glossary](docs/src/theory/glossary.md), [Decisions](docs/src/theory/decisions.md)
- [Architecture](docs/src/architecture.md)
- [Content Model](docs/src/content-model.md)
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
| `taxus-domain` | The Site Tree, identity types, frontmatter schema and pure derivations; no I/O |
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