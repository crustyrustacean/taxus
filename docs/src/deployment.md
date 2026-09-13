# Deployment

A built taxus site is a static directory — any host that serves files
can deploy it. This page covers what the build produces, the options
that matter for deployment, and the common hosts.

## What gets built

`taxus build` writes the site to the output directory (default `dist/`,
configurable as `[build] output_dir`):

```
dist/
├── index.html            # and one <page>/index.html per page
├── blog/…                # section pages, term pages, page/2/ pagination
├── css/                  # compiled SCSS
├── static/               # copied verbatim from static/
├── images/               # hero image variants (WebP/srcset)
├── wasm/                 # islands hydration client (client.js, client_bg.wasm)
├── search_index.bin      # client-side search index
├── feed.xml, feed.atom   # RSS and Atom feeds (if enabled)
├── sitemap.xml, robots.txt
└── 404.html              # alias redirects write more <alias>/index.html
```

Everything is pre-rendered HTML. A page is readable with JavaScript
disabled; islands hydrate progressively in browsers that run the WASM
client.

## Options that matter for deployment

### Disabling islands and search

A site that uses no `island()` call in its templates can skip shipping
several hundred KB of WASM and search index. Set in `site.toml`:

```toml
[build]
islands = false   # skip dist/wasm/
search = false    # skip dist/search_index.bin
```

`taxus init --no-islands` writes `islands = false` for you. Both
default to `true`.

### Drafts

Drafts are excluded by default. `taxus build --include-drafts` includes
them — useful for a preview deployment of unreleased content, and a
mistake for production.

### Base URL

`site.base_url` is baked into absolute URLs: permalinks, feeds, the
sitemap, canonical links. It must match where the site is actually
hosted (`https://example.com`, or `https://user.github.io/repo` for
project sites).

## Hosts

### Any static file host

Upload the contents of `dist/` — the directory itself, not a parent.
The site has no server-side component: no PHP, no server functions, no
environment variables.

Two files deserve host-specific attention:

- **`404.html`** — most static hosts let you designate a custom 404
  page; point it at this file.
- **`feed.xml` / `feed.atom`** — served as-is; the correct
  `Content-Type` (`application/rss+xml` / `application/atom+xml`) is a
  nicety most hosts get right or that readers tolerate without.

### Cloudflare Pages

This is how [get-taxus.org](https://crustyrustacean.github.io/get-taxus-org/)
is deployed (see [Development](./development.md) for the repo's own
workflow):

1. Build command: `taxus build` (or `cargo run --release -- build` in CI)
2. Output directory: `dist`
3. That's the whole configuration.

### GitHub Pages

Two shapes:

**Action-based (recommended):** a workflow checks out, builds with
`taxus build`, and uploads `dist/`:

```yaml
- uses: actions/checkout@v4
- run: taxus build   # or build from source in CI
- uses: actions/upload-pages-artifact@v3
  with:
    path: dist
```

**Branch-based:** build locally, push `dist/` to the `gh-pages` branch
(a tool like `git subtree push` works), and set Pages to that branch.
Remember to set `base_url` to the project-site path
(`https://user.github.io/repo`) — pages link relatively, so the site
works from a subdirectory.

### Netlify / Vercel

Build command `taxus build`, publish directory `dist`. On Netlify add a
redirect for the 404:

```toml
# netlify.toml
[build]
  command = "taxus build"
  publish = "dist"

[[redirects]]
  from = "/*"
  to = "/404.html"
  status = 404
```

## Installing the taxus binary

The release workflow builds the `taxus` binary for six platforms
(macOS, Linux and Windows, ARM64 and x86-64). Every GitHub release
carries the archives plus generated one-line installers:

**macOS / Linux:**

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/crustyrustacean/taxus/releases/latest/download/taxus-installer.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy ByPass -c \
  "irm https://github.com/crustyrustacean/taxus/releases/latest/download/taxus-installer.ps1 | iex"
```

Both install into `~/.cargo/bin` (the install path is `CARGO_HOME`), so
the binary sits beside `cargo` and is found if that directory is on
your `PATH`.

Building from source remains supported — see the
[README](https://github.com/crustyrustacean/taxus#installation) — as is
checking out a specific release tag first (`git checkout vX.Y.Z`) when
you want to pin.
