+++
title = "Taxus Feature Focus: Syntax Highlighting"
date = 2026-04-10
description = "A deep dive on how Taxus handles syntax highlighting."
draft = false
+++

Taxus is evolving into a *toolkit* for composing a static site generator. I started this project out with the notion it would be a competitor to [Zola](https://getzola.org) but as it evolved, given the choices made, that path is abandoned. You can certainly use Taxus to *create* a single binary on par with Zola, but the WASM functionality I'm experimenting with necessitates abandoning the notion of a single binary.

Taxus is not yet on par with Zola. There are a variety of quality of life things that haven't been achieved, both in terms of features and just general functionality.

Today we focus on syntax highlighting, the first big missing feature..

## 1. The Problem

Syntax highlighting is table-stakes for any technical blog where an author wants to write and talk about code and code snippets. Taxus needed to embody a solid foundation for the future.  Most Rust static site generators grab `syntect` and call it a day. `syntext` is a syntax highlighting library for Rust that uses [Sublime text syntax definitions](https://www.sublimetext.com/docs/syntax.html#include-syntax). Essentially, it relies on regex-based token matching, which is fine generally, but is a bit limiting because regex grammars struggle with Rust-specific syntax, namely lifetimes, turbofish, macro invocations and nested generics with trait bounds.

Taxus is going to do better.

Taxus uses [tree-sitter](https://tree-sitter.github.io/tree-sitter/).

## 2. Why Tree-Sitter

Regex, as powerful and ubiquitous as it is, doesn't understand *relationships* inherent in code. By nature, `tree-sitter` can "understand" code structure more deeply and this resolves the inherent brittleness and maintenance difficulty in using Regex.  `tree-sitter` produces a full *abstract syntax tree* and doesn't just process a token stream. It understands structure, it knows:

- `'a' is a lifetime, not a character literal followed by an identifier
- turbofish (`::<Type>`) parses correctly because the underlying grammar knows about generic arguments in method position

The same grammars in `tree-sitter` powewr syntax highlighting in Neovim, Helix, and Zed -- so it's been battle-tested on real Rust code.

## 3. Architecture Decisions

- Tree-sitter and tree-sitter-highlight are always-on dependencies; individual language grammars are behind feature flags (`lang-rust`, etc.)
- Rust is enabled by default, future languages opt-in via `--features lang-toml`, `lang-typescript`, etc.
- Highlight queries (`.scm` files) are bundled via `include_str!` — no runtime file loading
- A `LanguageRegistry` maps language names and aliases to grammar + query pairs
- `HighlightConfiguration` is built once at startup and reused across all code blocks for performance

## 4. The Highlight Pipeline

- Pulldown-cmark parses Markdown and emits events
- Custom event loop intercepts `CodeBlock` events instead of using `push_html`
- Fenced blocks with a language tag go through tree-sitter; everything else falls through to plain `<pre><code>`
- Output is semantic `<span>` tags with CSS classes (`hl-keyword`, `hl-type-builtin`, etc.) — no inline styles
- Colors are entirely controlled by a CSS theme file, making it trivial to swap themes without rebuilding

## 5. What It Gets Right That Syntect Doesn't

- Show real output comparisons for:
  - Lifetimes: `'a` as a single highlighted unit vs split tokens
  - Turbofish: `.parse::<u32>()` with correct type highlighting
  - Macro invocations: `format!()` scoped as `function.macro`
  - Impl blocks with where clauses: types, traits, and keywords all correctly distinguished
  - `self` recognized as a builtin variable, not a regular identifier

## 6. Theme System

- Two built-in SCSS partials: light (GitHub-inspired) and dark (Catppuccin-inspired)
- Users swap themes by changing a single `@import` line
- Custom themes are just CSS mapping the `hl-*` classes — no tooling required
- Both themes ship with `taxus init` so highlighting works out of the box

## 7. Configuration

- Enabled by default, zero-config for the common case
- Configurable via `site.toml`: `enabled`, `class_prefix`
- When disabled, code blocks render as plain `<pre><code>` — no tree-sitter overhead

## 8. What's Next

- The highlighting foundation sets up a future feature: interactive Rust playground islands
- Static code blocks are SSR'd with tree-sitter at build time; a Yew WASM island can hydrate them into a runnable playground in the browser
- This is only possible because Taxus already has an islands architecture — something no other Rust SSG offers

## 9. Try It

- Link to the repo
- Quick start: `taxus init my-site && cd my-site && taxus serve`
- Drop a Rust code block into any Markdown file and see it highlighted