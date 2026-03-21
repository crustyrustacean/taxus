# Yew SSG — justfile
#
# Requires `just` (https://github.com/casey/just)
# Install: cargo install just
#
# Usage:
#   just build my-site             # build SSG pages + WASM client for a site
#   just serve my-site             # build + serve with miniserve
#   just init my-site              # scaffold a new site
#   just wasm my-site              # compile WASM client only
#   just pages my-site             # build SSG pages only
#   just clean my-site             # remove dist/
#   just routes my-site            # show discovered routes
#   just test                      # run all tests
#   just check                     # cargo check without building
#   just doc                       # open generated API docs
#
# All recipe site arguments default to "." (current directory).

# set Powershell for Windows
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# ─── Full build ───────────────────────────────────────────────────────────────

# Plain SSG build — no Yew/WASM dependency, island() calls produce empty output
build site=".":
    just pages {{site}}

# Islands build — Yew SSR + WASM client (requires --features islands)
build-islands site=".":
    just pages-islands {{site}}
    just wasm {{site}}

# ─── Individual stages ────────────────────────────────────────────────────────

# Build static HTML pages (plain SSG, no islands)
pages site=".":
    cargo run -- build --dir {{site}} --verbose

# Build static HTML pages with Yew SSR islands enabled
pages-islands site=".":
    cargo run --features islands -- build --dir {{site}} --verbose

# Compile the Yew WASM hydration client and write into <site>/dist/wasm/
wasm site=".":
    cd client && trunk build --release --dist ../{{site}}/dist/wasm

# ─── Development helpers ──────────────────────────────────────────────────────

# Build then serve <site>/dist with miniserve on port 8080
serve site=".": (build site)
    miniserve {{site}}/dist --port 8080 --index index.html

# Scaffold a new site (plain SSG, no islands)
init site="my-site" name="My Site" url="https://example.com":
    cargo run -- init {{site}} --name "{{name}}" --base-url "{{url}}"

# Scaffold a new site with islands support (Yew/WASM)
init-islands site="my-site" name="My Site" url="https://example.com":
    cargo run -- init {{site}} --name "{{name}}" --base-url "{{url}}" --islands

# Remove the generated dist/ directory for the site
clean site=".":
    cargo run -- clean --dir {{site}}

# Show all routes discovered in <site>/content without building
routes site=".":
    cargo run -- routes --dir {{site}}

# ─── Repository-wide tasks ────────────────────────────────────────────────────

# Run all tests
test:
    cargo test

# Check compilation without producing binaries
check:
    cargo check

# Generate and open API documentation
doc:
    cargo doc --open

# Release build — plain SSG (no islands, smallest binary)
release site=".":
    cargo build --release
    target/release/yew-ssg build --dir {{site}} --verbose

# Release build — islands mode (Yew SSR + WASM client)
release-islands site=".":
    cargo build --release --features islands
    target/release/yew-ssg build --features islands --dir {{site}} --verbose
    just wasm {{site}}
