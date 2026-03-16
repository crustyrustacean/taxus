# Development

This guide covers development workflows and best practices for Yew SSG.

## Development Setup

### Prerequisites

1. **Rust**: Install from [rustup.rs](https://rustup.rs/)
2. **trunk**: WebAssembly bundler
   ```bash
   cargo install trunk
   ```
3. **mdbook**: Documentation generator (optional)
   ```bash
   cargo install mdbook
   ```

### Clone and Build

```bash
git clone https://github.com/crustyrustacean/yew-ssg.git
cd yew-ssg
cargo build
```

## Development Workflow

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p generator

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test config_loading
```

### Building the Site

```bash
# Build static site
cargo run
```

### Client Development

For client-side development with hot-reload:

```bash
cd client
trunk serve
```

This starts a development server at `http://localhost:8080`.

### Documentation

Build and serve the documentation:

```bash
cd docs
mdbook serve
```

Open `http://localhost:3000` to view the documentation.

## Project Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Build all crates |
| `cargo test` | Run all tests |
| `cargo run` | Build the static site |
| `cargo doc` | Generate API documentation |
| `cargo clippy` | Run linter |
| `cargo fmt` | Format code |

## Code Organization

### Workspace Structure

The project uses a Cargo workspace with three crates:

```
Cargo.toml          # Workspace manifest
client/             # WebAssembly client
common/             # Shared components
generator/          # SSG library and binary
```

### Adding Dependencies

Add dependencies to the appropriate crate's `Cargo.toml`:

```toml
# generator/Cargo.toml
[dependencies]
thiserror = "2.0"
```

For workspace-wide dependencies, add to the root `Cargo.toml`:

```toml
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
```

Then reference in crate manifests:

```toml
[dependencies]
serde = { workspace = true }
```

## Testing

### Unit Tests

Unit tests are placed in the source files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test code
    }
}
```

### Integration Tests

Integration tests go in the `tests/` directory:

```
generator/
└── tests/
    ├── config_loading.rs
    └── fixtures/
        ├── minimal_site/
        └── full_site/
```

### Test Fixtures

Use test fixtures for integration tests:

```rust
#[test]
fn test_load_config() {
    let config = SiteConfig::from_dir("tests/fixtures/minimal_site").unwrap();
    assert_eq!(config.site.name, "Minimal Site");
}
```

## Debugging

### Logging

Add logging to your code:

```rust
// Future: Add tracing support
use tracing::{info, debug};

fn build_site() {
    info!("Building site");
    debug!("Processing content");
}
```

### Error Messages

Use descriptive error messages:

```rust
let config = SiteConfig::from_dir(".")
    .map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        e
    })?;
```

## Release Process

### Build for Production

```bash
# Build in release mode
cargo build --release

# Build client for production
cd client && trunk build --release
```

### Generate Documentation

```bash
# Generate API docs
cargo doc --no-deps

# Build mdbook
cd docs && mdbook build
```

## Contributing

### Code Style

1. Run `cargo fmt` before committing
2. Run `cargo clippy` and fix warnings
3. Add tests for new functionality
4. Update documentation

### Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Run linter: `cargo clippy`
6. Format code: `cargo fmt`
7. Submit a pull request

## IDE Setup

### VS Code

Recommended extensions:

- rust-analyzer
- CodeLLDB
- Better TOML
- Markdown All in One

### Configuration

Create `.vscode/settings.json`:

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all"
}
```
