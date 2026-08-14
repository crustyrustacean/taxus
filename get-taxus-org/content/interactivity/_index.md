+++
title = "Interactivity"

[extra]
counter = true
+++

### Interactivity Two Ways

Taxus provides two options to make your static site interactive. Use them independently or together.

| Layer | Technology | Best For |
|-------|------------|----------|
| General | `static/scripts.js` | DOM manipulation, toggles, analytics |
| Performance | Yew WASM islands | Complex state, heavy computation |

### Plain 'ol JavaScript

Taxus outputs an empty `scripts.js` for you to fill as you see fit. Use it for simple interactivity across your site.

### WebAssembly with Yew

Islands are a first-class, always-on part of Taxus: every build ships the WASM hydration client, and scaffolds initialize with islands enabled (pass `--no-islands` to `init` for a plain Tera/Markdown site). With the power of [Yew](https://yew.rs) you can create widgets that tap into WebAssembly performance. Pages render immediately with pre-rendered HTML—WASM hydrates components asynchronously.

Here's a counter widget to demonstrate:

### Documentation

For complete islands reference, see the [Islands Architecture documentation](https://crustyrustacean.github.io/taxus/islands.html).
