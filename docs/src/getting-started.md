# Getting Started

This guide will help you get up and running with Taxus quickly.

## Prerequisites

Install the `taxus` binary (macOS, Linux and Windows; ARM64 and x86-64):

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/crustyrustacean/taxus/releases/latest/download/taxus-installer.sh | sh
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/crustyrustacean/taxus/releases/latest/download/taxus-installer.ps1 | iex"
```

The installer puts `taxus` in `~/.cargo/bin`, beside `cargo`. See
[Deployment](./deployment.md#installing-the-taxus-binary) for details.
Building from source is covered at the end of this page.

## Quick Start

### Step 1: Initialize

```bash
taxus init my-site --name "My Site" --base-url "https://example.com"
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
    ├── main.scss           # Starter stylesheet
    ├── _highlight-dark.scss  # Code highlighting theme (dark)
    └── _highlight-light.scss # Code highlighting theme (light)
```

### Step 2: Build the Site

```bash
taxus build --dir my-site --verbose
```

This runs the 15-stage build pipeline (see [Architecture](./architecture.md)) and writes output to `my-site/dist/`. The WASM client (`client.js` and `client_bg.wasm`) is compiled during the Cargo build, embedded in the binary, and written to `dist/wasm/` automatically — no separate build step is needed.

### Step 3: Serve and View

```bash
taxus serve --dir my-site --open
```

This starts a development server at `http://localhost:3000` and opens it in your browser.

You should see the home page rendered from the Markdown content in
`content/_index.md`. The scaffold's `section.html` also renders a working
`Counter` island: a button that increments once the WASM client hydrates
it. Delete the `island(...)` line from `templates/section.html` — or
initialize with `--no-islands` — for a site without it.

## Next Steps

- Read the [Theory](./theory/overview.md) chapters to learn how a build works
- Learn about [Configuration](./configuration.md) for customizing your site
- Understand [Content](./content.md) for writing pages and posts
- Explore [Templates](./templates.md) for customizing HTML output
- Read the [CLI Reference](./cli.md) for all command options
- [Deploy](./deployment.md) the built site to a static host

## Opting Out of Islands

Islands (Yew/WASM hydration) are enabled by default. If you want a plain
Tera/Markdown scaffold with no WASM hydration, pass `--no-islands` when
initializing:

```bash
taxus init my-site --no-islands
```

See [Islands Architecture](./islands.md) for the complete guide.

## Building Taxus from Source

You need the repository only to develop Taxus itself or to pin an
unreleased commit:

```bash
git clone https://github.com/crustyrustacean/taxus.git
cd taxus
cargo build --release
# the binary is target/release/taxus
```

Rust (edition 2024) is required —
[Install Rust](https://www.rust-lang.org/tools/install). Run the
commands above as `cargo run -- build --dir my-site` and so on instead
of `taxus build`. The WASM client is compiled and embedded by a build
script automatically, so a source build behaves exactly like the
released binary.
