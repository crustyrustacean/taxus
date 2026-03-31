# Development

This guide covers development workflows for contributing to Yew SSG.

## Prerequisites

- **Rust** — [Install Rust](https://rustup.rs/)
- **trunk** — WebAssembly bundler: `cargo install trunk`
- **mdbook** — Documentation: `cargo install mdbook`

## Setup

```bash
git clone https://github.com/crustyrustacean/yew-ssg.git
cd yew-ssg
cargo build
```

## Running Tests

```bash
# Run all tests (238+ tests)
cargo test

# Run tests for a specific crate
cargo test -p generator

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test config_loading
```

## Building

```bash
# Build static site (plain SSG)
cargo run -- build --dir my-site

# Build with islands support
cargo run --features islands -- build --dir my-site

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
RUST_LOG=yew_ssg_lib=trace cargo run -- build
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
yew-ssg/
├── client/     # WASM hydration client
├── common/     # Shared Yew components
├── generator/  # SSG library and CLI
└── docs/       # mdBook documentation
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes
4. Run tests: `cargo test`
5. Run linter: `cargo clippy`
6. Format: `cargo fmt`
7. Submit a pull request
