+++
title = "Taxus Feature Focus: Search"
date = 2026-04-10
description = "A deep dive on how Taxus handles search."
draft = true
+++

**Title:** Building a WASM-Ready Search Engine for a Static Site Generator in Rust

**Introduction**
- The problem: static sites have no server to handle search queries
- The approach: build the index at compile time, query it at runtime in the browser
- Why Rust end-to-end: the same tokenizer and data structures compile to both native (for the generator) and WASM (for the browser)

**The Architecture**
- Three crates, two targets: `taxus-common` holds the search engine, `taxus-generator` produces the index, `taxus-client` will consume it
- The shared crate is the key — identical logic at build time and query time eliminates mismatches

**The Search Engine**
- Tokenizer: lowercasing, splitting on non-alphanumeric characters, filtering short tokens
- Stemmer: collapsing word variants with Snowball via `rust-stemmers`
- Inverted index: a HashMap mapping stemmed tokens to document IDs
- TF-IDF scoring: term frequency weighted by inverse document frequency
- The `finalize` step: why scoring can only happen after all documents are indexed

**The Pipeline Integration**
- Following the existing pattern: `generate_` and `write_` functions mirroring feeds and sitemap stages
- Feature-gating behind `islands` since the consumer is WASM
- Output: a compact binary file via `postcard` serialization

**What's Next**
- Phase 3: loading the index in the browser via Fetch API and exposing search through `wasm-bindgen`
- Phase 4: a Yew island component — a search box that hydrates client-side
- The JSON escape hatch: swapping `postcard` for JSON to support pure-JS consumers without changing the engine

**Reflections on the Process**
- Patterns that repeated: the check-then-insert HashMap pattern appeared four times across different contexts
- Building in small testable steps: each piece worked in isolation before being composed
- The value of shared types: one tokenizer, one stemmer, one set of data structures across the entire system