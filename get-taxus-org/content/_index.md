+++
title = "Home"
+++

## Static Site Generation for 2026 and beyond

Building for the web has become unnecessarily difficult. Taxus is an opinionated take on making it easier. It takes in assets and markdown and outputs a fully complete website, ready for deployment.

## Pillars

Taxus stands on the following pillars:

- author with power
    - write with common-mark compliant markdown, via [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark)
- structure is easy and familiar
    - HTML templates with [Tera](https://keats.github.io/tera)
- appearance is yours to determine
    - flexible SASS compliation with [grass](https://github.com/connorskees/grass)
- interactivity is simple, performant, or both
    - JavaScript via ready-made `scripts.js`
    - WebAssembly via the "islands" architecture, enabled with [Yew](https://yew.rs) components

## Foundations

Taxus stands on several foundational crates from the Rust ecosystem, including [Tokio](https://tokio.rs/), [Axum](https://github.com/tokio-rs/axum) and many others.

## License

The project is MIT licensed. Take it, modify it, use it as you see fit and need.
