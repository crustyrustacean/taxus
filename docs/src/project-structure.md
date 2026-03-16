# Project Structure

The Yew SSG project is organized as a multi-crate Rust workspace. This structure separates concerns and enables code reuse between the client, common components, and the generator.

## Directory Overview

```
yew-ssg/
├── client/              # Client-side WebAssembly application
├── common/              # Shared components and code
├── generator/           # Static site generator
├── content/             # Markdown content files
├── static/              # Static assets
├── styles/              # SCSS stylesheets
├── templates/           # HTML templates
├── docs/                # Documentation (mdbook)
└── .plans/              # Planning documents
```

## Workspace Crates

### client

The WebAssembly client application built with Yew.

```
client/
├── Cargo.toml
└── src/
    └── main.rs          # Application entry point
```

This crate contains the client-side WebAssembly application that hydrates the static HTML with interactive components.

### common

Shared components and utilities used by both client and generator.

```
common/
├── Cargo.toml
└── src/
    ├── lib.rs           # Module exports
    └── components/
        ├── mod.rs
        ├── about.rs     # About page component
        ├── home.rs      # Home page component
        ├── layout.rs    # Layout wrapper
        └── page.rs      # Generic page component
```

This crate defines:
- The `Route` enum for navigation
- Reusable page components
- Layout and styling shared between SSR and client

### generator

The static site generator library and binary.

```
generator/
├── Cargo.toml
├── lib.rs               # Empty (legacy)
└── src/
    ├── lib.rs           # Library entry point
    ├── config.rs        # Configuration types
    ├── error.rs         # Error handling
    └── bin/
        └── main.rs      # CLI binary
```

The generator crate provides:
- **Library**: Reusable SSG functionality with configuration and error handling
- **Binary**: CLI tool that pre-renders pages

See the [Generator Library](./generator/README.md) section for more details.

## Content Directory

Markdown content files for your site.

```
content/
└── pages/
    ├── home.md          # Home page content
    └── about.md         # About page content
```

Each Markdown file should include frontmatter:

```markdown
+++
title = "Page Title"
+++

# Page Content

Your markdown content here.
```

## Static Assets

Static files that are copied directly to the output:

```
static/
├── favicon.png          # Site favicon
└── scripts.js           # Additional JavaScript
```

## Styles

SCSS stylesheets that are compiled to CSS:

```
styles/
└── styles.scss          # Main stylesheet
```

## Templates

HTML templates used for rendering:

```
templates/
└── index.txt            # Base template
```

## Output

Generated files are placed in the `dist/` directory:

```
dist/
├── index.html           # Home page
├── about/               # About page directory
│   └── index.html
├── css/                 # Compiled CSS
│   └── styles.css
└── favicon.png          # Copied static files
```
