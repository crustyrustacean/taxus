# Getting Started

This guide will help you get up and running with Yew SSG quickly.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (edition 2024) - [Install Rust](https://www.rust-lang.org/tools/install)
- **trunk** - WebAssembly bundler for Rust - [Install trunk](https://trunkrs.dev/)
- **A SCSS compiler** (optional, for styling) - e.g., `sass`

## Quick Start

### Option A: Initialize a New Site

Create a new site from scratch:

```bash
# Clone the repository
git clone https://github.com/crustyrustacean/yew-ssg.git
cd yew-ssg

# Initialize a new site
cargo run -- init my-site --name "My Site" --base-url "https://example.com"

# Navigate to the new site
cd my-site

# Build the site
cargo run -- build
```

### Option B: Use the Example Site

Build the existing example site:

```bash
# Clone the repository
git clone https://github.com/crustyrustacean/yew-ssg.git
cd yew-ssg

# Build the static site
cargo run -- build
```

This will:
1. Read Markdown content from `content/`
2. Process SCSS styles from `styles/`
3. Generate HTML files in the `dist/` directory

### View the Generated Site

You can serve the generated files using any static file server:

```bash
# Using Python
python -m http.server 8000 -d dist

# Using Node.js (npx)
npx serve dist

# Or simply open dist/index.html in your browser
```

## Development Mode

For development with hot-reloading of the client-side WebAssembly application:

```bash
cd client && trunk serve
```

This will:
- Compile the client to WebAssembly
- Start a development server with hot-reload
- Open your browser at `http://localhost:8080`

## Project Commands

| Command | Description |
|---------|-------------|
| `cargo run -- init [path]` | Initialize a new site |
| `cargo run -- build` | Build the static site |
| `cargo run -- build --verbose` | Build with verbose output |
| `cargo run -- build --include-drafts` | Build including drafts |
| `cargo test` | Run all tests |
| `cargo doc` | Generate API documentation |
| `cd client && trunk serve` | Start development server |
| `cd client && trunk build` | Build client for production |

## Next Steps

- Learn about the [Project Structure](./project-structure.md)
- Configure your site with [Configuration](./configuration.md)
- Explore the [Generator Library](./generator/README.md)
