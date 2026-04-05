# Introduction

Welcome to **Taxus**, a Rust-based static site generator built with [Tera](https://keats.github.io/tera/), featuring optional WebAssembly "islands" for interactive components.

## What is Taxus?

Taxus is a static site generator that combines:

- **Tera templates** for the static "sea of HTML" — page layout, content, navigation
- **Markdown content** with TOML frontmatter for writing pages and posts
- **SCSS** for modern styling
- **Optional Yew components** — pre-rendered server-side at build time, hydrated by WASM in the browser for interactivity

The key insight: pages are immediately visible with no JavaScript required. Interactive components load asynchronously and attach without re-rendering.

## Features

- **Static-first**: Pre-rendered HTML for optimal performance and SEO
- **Islands Architecture**: Interactive Yew WASM components embedded within static pages
- **Markdown Content**: Write content in Markdown files with TOML frontmatter
- **Co-located Assets**: Images and files in the content directory are automatically copied to output
- **Internal Links**: Reference other pages by content file path with build-time validation
- **Blog Features**: Summary extraction, reading time, word count, custom slugs
- **Pagination**: Split large collections across multiple pages
- **RSS/Atom Feeds**: Automatic feed generation for content syndication
- **Sitemap**: Automatic `sitemap.xml` generation with priorities and lastmod dates
- **Taxonomies**: Tags, categories, and series for content organization
- **Development Server**: Hot-reloading local server with WebSocket live reload
- **CLI Interface**: `build`, `clean`, `init`, `routes`, and `serve` subcommands

## Who is this for?

Taxus is ideal for:

- **Rust developers** who want to build websites without leaving their favorite language
- **Performance enthusiasts** who want fast, optimized static sites with selective WASM interactivity
- **SEO-conscious developers** who need pre-rendered content with no JavaScript dependency for initial render
- **Component lovers** who prefer the Yew component model for interactive UI pieces

## Documentation Overview

- [Getting Started](./getting-started.md) — Quick start guide
- [Architecture](./architecture.md) — Technical overview of the workspace, modules, and build pipeline
- [Configuration](./configuration.md) — `site.toml` format and options
- [Content](./content.md) — Markdown files, frontmatter, taxonomies, pagination
- [Templates](./templates.md) — Tera templates and context variables
- [Islands Architecture](./islands.md) — How to write and use Yew components
- [CLI Reference](./cli.md) — Command-line interface documentation
- [Development Server](./serve.md) — Hot reload and file watching
- [API Reference](./api-reference.md) — Library API documentation

## License

This project is licensed under the MIT License - see the [License.txt](../License.txt) file for details.
