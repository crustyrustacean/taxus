# Getting Started

This guide will help you get up and running with Yew SSG quickly.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (edition 2024) - [Install Rust](https://www.rust-lang.org/tools/install)
- **trunk** - WebAssembly bundler for Rust - [Install trunk](https://trunkrs.dev/)
- **A SCSS compiler** (optional, for styling) - e.g., `sass`

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/crustyrustacean/yew-ssg.git
cd yew-ssg
```

### 2. Build the Static Site

Run the generator to produce static HTML files:

```bash
cargo run
```

This will:
1. Read Markdown content from `content/pages/`
2. Process SCSS styles from `styles/`
3. Generate HTML files in the `dist/` directory

### 3. View the Generated Site

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
| `cargo run` | Build the static site |
| `cargo test` | Run all tests |
| `cargo doc` | Generate API documentation |
| `cd client && trunk serve` | Start development server |
| `cd client && trunk build` | Build client for production |

## Next Steps

- Learn about the [Project Structure](./project-structure.md)
- Configure your site with [Configuration](./configuration.md)
- Explore the [Generator Library](./generator/README.md)
