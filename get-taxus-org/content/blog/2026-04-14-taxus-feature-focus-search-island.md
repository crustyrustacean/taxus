+++
title = "Taxus Feature Focus: Search Island"
date = 2026-04-14
description = "Client-side full-text search powered by a Yew WASM island — no external service required."
tags = ["rust", "wasm", "yew", "search"]
categories = ["features"]
series = "Feature Focus"
+++

Static sites have long had a search problem. The typical solutions all come with tradeoffs: Algolia costs money and sends queries to a third party, Lunr.js adds a heavy JavaScript dependency, and Google Custom Search is — well — Google. None of these feel right for a site that proudly generates its own HTML at build time.

Taxus now ships a built-in alternative: a **Search Island** powered by Yew and WebAssembly.

## The Island Architecture

If you've read the earlier posts in this series, you know Taxus supports *islands* — interactive WebAssembly components that hydrate pre-rendered HTML. The search island follows this same pattern:

1. **Build time**: Taxus indexes all non-draft pages into a compact binary search index
2. **Serve time**: The index is embedded in the page as a static asset
3. **Run time**: A Yew WASM component loads the index and provides instant client-side search

No server round-trips. No third-party services. No API keys. Just a search box that works.

## How It Works

### The Search Index

During the build pipeline, Taxus walks every processed page and extracts:

- **Title** — weighted most heavily in results
- **Description** — secondary weight
- **Content** — the rendered HTML, stripped of markup
- **URL path** — for linking to results

The index is serialized with [postcard](https://docs.rs/postcard) into `search_index.bin` and written to the output directory alongside your other static assets. For a typical blog with a few hundred posts, the index is surprisingly small — usually under 100 KB, and gzip brings it well below that.

### The Yew Component

The search box is a Yew component rendered via the `{{ island(component="SearchBox") }}` template function. In `base.html`, you'll find it right in the header:

```html
<div class="search-container">
    {{ island(component="SearchBox") | safe }}
</div>
```

When the WASM module hydrates, the search box becomes interactive. Typing a query filters the index client-side and displays results instantly — no network request needed.

### Fuzzy Matching

The search engine uses a simple but effective tokenization strategy:

- Queries are lowercased and split on whitespace
- Each token is looked up in the TF-IDF index built at build time: terms that are common across all pages count for less, distinctive terms count for more
- Results are ranked by summed TF-IDF scores across the matched tokens
- Whole-word matches only — the index is token-based, so type the full word you're looking for

This isn't meant to compete with Elasticsearch. It's meant to give your readers a fast, reliable way to find content without leaving the page.

## Setup

The search island is included automatically when you scaffold a new site with `taxus init`. For existing sites, add the component to your templates:

```html
<div class="search-container">
    {{ island(component="SearchBox") | safe }}
</div>
```

And ensure the WASM client is loaded in your `base.html`:

```html
<script type="module">
    import init, * as bindings from '/wasm/client.js';
    const wasm = await init({ module_or_path: '/wasm/client_bg.wasm' });
    window.wasmBindings = bindings;
</script>
```

That's it. The search index is generated at build time, the WASM component hydrates at run time, and your readers get instant search.

## Why Not Just Use JavaScript?

You could. A Lunr.js integration would work fine. But the island approach has real advantages:

- **Type safety**: The search component is written in Rust, compiled to WASM. Refactoring catches bugs at compile time, not in production.
- **Shared types**: The same `Page` and `ProcessedPage` types flow through the build pipeline and the search component. No drift between your index schema and your component code.
- **Consistent architecture**: Taxus already has the WASM runtime for the Counter island and future interactive features. Search reuses the same infrastructure — no additional runtime cost.
- **No NPM**: Your static site doesn't need a JavaScript package manager. The WASM is compiled by Trunk and served as static files.

## Performance

Client-side search has a reputation for being slow on large sites. The reality is more nuanced:

- **Index load**: The binary index loads once, on page load, as a static asset. The browser caches it.
- **Query speed**: Filtering a few hundred entries in WASM is effectively instant — sub-millisecond on modern hardware.
- **Bundle size**: The WASM module adds ~50 KB gzipped to your page. That's less than most analytics scripts.

For sites with thousands of pages, you'd want server-side search. For blogs and documentation sites — Taxus's sweet spot — client-side search is the right tradeoff.

## What's Next

The search island is a foundation for future improvements:

- **Stemming and stopwords**: Smarter tokenization for more relevant results
- **Search result previews**: Snippet extraction showing the matching context
- **Keyboard navigation**: Arrow keys and Enter to navigate results without touching the mouse
- **Search analytics**: Optional metrics on what visitors are searching for (self-hosted, of course)

Search is available now in Taxus. Add the island to your templates, rebuild, and give your readers a way to find what they're looking for.

---

*This is part of the Feature Focus series, covering Taxus capabilities in depth. Previous entries: [Syntax Highlighting](/blog/2026-04-10-taxus-feature-focus-syntax-highlighting/) and [Hero Images](/blog/2026-04-11-taxus-feature-focus-hero-images/).*
