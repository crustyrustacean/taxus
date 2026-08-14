# Development

This guide covers development workflows for contributing to Taxus.

## Prerequisites

- **Rust** — [Install Rust](https://rustup.rs/)
- **mdbook** ≥ 0.5.0 — Documentation: `cargo install mdbook --locked`

## Setup

```bash
git clone https://github.com/crustyrustacean/taxus.git
cd taxus

cargo build
```

## Running Tests

```bash
# Run all tests (600+)
cargo test

# Run tests for a specific crate
cargo test -p taxus

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test config_loading
```

## Building

```bash
# Build the site (islands and the WASM client are compiled and embedded automatically)
cargo run -- build --dir my-site

# Build release binary
cargo build --release
```

## Development Server

```bash
# Start server with auto-reload
cargo run -- serve --dir my-site --open
```

## Documentation

```bash
cd docs
mdbook serve
```

Open `http://localhost:3000` to view.

## Code Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Build all crates |
| `cargo test` | Run all tests |
| `cargo run -- build` | Build the static site |
| `cargo run -- serve` | Start dev server |
| `cargo doc` | Generate API docs |
| `cargo clippy` | Run linter |
| `cargo fmt` | Format code |

## `xtask` Task Runner

The workspace includes an `xtask` crate (aliased as `cargo xtask` via
`.cargo/config.toml`) that wraps common developer workflows:

| Command | Description |
|---------|-------------|
| `cargo xtask build [--release] [--features ...]` | Build the project |
| `cargo xtask test [--release] [--nextest] [--features ...]` | Run unit and integration tests |
| `cargo xtask check [--features ...]` | Fast compile check (no codegen) |
| `cargo xtask lint [--features ...] [--fix]` | Lint with Clippy |
| `cargo xtask fmt [--check]` | Check formatting with rustfmt |
| `cargo xtask doc [--open]` | Build Rust documentation |
| `cargo xtask book [--serve]` | Build the mdBook documentation in `docs/` |
| `cargo xtask audit` | Run `cargo audit` security scan (requires `cargo-audit`) |
| `cargo xtask wasm [--release]` | Build WASM artifacts |
| `cargo xtask clean` | Clean build artifacts |
| `cargo xtask ci` | Run the full local CI pipeline (fmt, lint, test) |
| `cargo xtask release --bump <major|minor|patch> [--dry-run]` | Prepare a release: changelog, tag, verify build |
| `cargo xtask deploy [--project <name>] [--branch <name>] [--prod-branch <name>] [--no-build]` | Build `get-taxus-org/` and deploy to Cloudflare Pages via wrangler (workspace tool; requires Cloudflare credentials) |

## Logging

Control log output with CLI flags or `RUST_LOG`:

```bash
# Default: info level
cargo run -- build

# Verbose: debug level
cargo run -- build --verbose

# Quiet: errors only
cargo run -- build --quiet

# Custom via RUST_LOG
RUST_LOG=debug cargo run -- build
RUST_LOG=taxus_lib=trace cargo run -- build
```

Add logging to code:

```rust
use tracing::{info, debug, warn, error};

fn build_site() {
    info!("Building site");
    debug!("Processing content");
    
    // Structured fields
    info!(pages = 5, sections = 2, "Build complete");
}
```

## Workspace Structure

```
taxus/
├── taxus-client/    # WASM hydration client
├── taxus-common/    # Shared Yew components
├── taxus-generator/ # SSG library and CLI
├── xtask/           # Workspace task runner (`cargo xtask`)
└── docs/            # mdBook documentation
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes
4. Run tests: `cargo test`
5. Run linter: `cargo clippy`
6. Format: `cargo fmt`
7. Submit a pull request