+++
title = "Interactivity"
description = "Add interactivity to a Taxus site with TypeScript, plain JavaScript, or WebAssembly islands powered by Yew — lightweight, performant, and dependency-free."

[extra]
counter = true
+++

### Interactivity Two Ways

Taxus provides two options to make your static site interactive. Use them independently or together.

| Layer | Technology | Best For |
|-------|------------|----------|
| General | TypeScript modules or `static/scripts.js` | DOM manipulation, toggles, analytics |
| Performance | Yew WASM islands | Complex state, heavy computation |

### TypeScript or Plain JavaScript

For new sites, TypeScript is the recommended general-purpose option: it is
checked at build time, emits browser-native modules, and needs no Node.js or
npm. Initialize with `taxus init --typescript`, or enable it manually in an
existing site with `[typescript] enabled = true` and a `tsconfig.json`.

TypeScript is a compiler, not a bundler. Use browser-resolvable relative
imports with the `.js` extension; npm package bundling is outside Taxus's
scope. Source maps are enabled by default in the generated scaffold.

Plain JavaScript remains a good fit for a tiny one-off behavior, and existing
sites can keep their `static/scripts.js` entrypoint while migrating.

### WebAssembly with Yew

Islands are a first-class, always-on part of Taxus: every build ships the WASM hydration client, and scaffolds initialize with islands enabled (pass `--no-islands` to `init` for a plain Tera/Markdown site). With the power of [Yew](https://yew.rs) you can create widgets that tap into WebAssembly performance. Pages render immediately with pre-rendered HTML—WASM hydrates components asynchronously.

Here's a counter widget to demonstrate:

### Documentation

For complete islands reference, see the [Islands Architecture documentation](https://crustyrustacean.github.io/taxus/islands.html).
