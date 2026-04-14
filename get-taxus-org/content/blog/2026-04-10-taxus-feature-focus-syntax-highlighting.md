+++
title = "Taxus Feature Focus: Syntax Highlighting"
date = 2026-04-10
description = "A deep dive on how Taxus handles syntax highlighting."
tags = ["rust", "tree-sitter", "syntax-highlighting"]
categories = ["features"]
series = "Feature Focus"
hero_image = "syntax_highlighting.jpg"
hero_alt = "Coloured code streaming across the screen, illustrating syntax highlighting"
+++

Taxus is evolving into a *toolkit* for composing a static site generator. I started this project out with the notion it would be a competitor to [Zola](https://getzola.org) but as it evolved, given the choices made, that path is abandoned. You can certainly use Taxus to *create* a single binary on par with Zola, but the WASM functionality I'm experimenting with necessitates abandoning the notion of a single binary.

Taxus is not yet on par with Zola. There are a variety of quality of life things that haven't been achieved, both in terms of features and just general functionality.

Today we focus on syntax highlighting, the first big missing feature.

## The Problem

Syntax highlighting is table-stakes for any technical blog where an author wants to write and talk about code and code snippets. Taxus needed to embody a solid foundation for the future. Most Rust static site generators grab `syntect` and call it a day. `syntect` is a syntax highlighting library for Rust that uses [Sublime Text syntax definitions](https://www.sublimetext.com/docs/syntax.html#include-syntax). Essentially, it relies on regex-based token matching, which is fine generally, but is a bit limiting because regex grammars struggle with Rust-specific syntax: lifetimes, turbofish, macro invocations, and nested generics with trait bounds.

Taxus does better.

Taxus uses [tree-sitter](https://tree-sitter.github.io/tree-sitter/).

## Why Tree-Sitter

Regex, as powerful and ubiquitous as it is, doesn't understand *relationships* inherent in code. By nature, tree-sitter can "understand" code structure more deeply, and this resolves the inherent brittleness and maintenance difficulty of using regex. Tree-sitter produces a full *abstract syntax tree* rather than just processing a token stream. It understands structure—it knows:

- `'a` is a lifetime, not a character literal followed by an identifier
- turbofish (`::<Type>`) parses correctly because the underlying grammar knows about generic arguments in method position

The same grammars in tree-sitter power syntax highlighting in Neovim, Helix, and Zed—so they've been battle-tested on real Rust code.

## Architecture Decisions

Tree-sitter and tree-sitter-highlight are always-on dependencies; individual language grammars are behind feature flags (`lang-rust`, etc.). Rust is enabled by default; future languages opt-in via `--features lang-toml`, `lang-typescript`, and so on.

Highlight queries (`.scm` files) are bundled via `include_str!`—no runtime file loading. A `LanguageRegistry` maps language names and aliases to grammar + query pairs. `HighlightConfiguration` is built once at startup and reused across all code blocks for performance.

## The Highlight Pipeline

Pulldown-cmark parses Markdown and emits events. A custom event loop intercepts `CodeBlock` events instead of using `push_html`. Fenced blocks with a language tag go through tree-sitter; everything else falls through to plain `<pre><code>`. Output is semantic `<span>` tags with CSS classes (`hl-keyword`, `hl-type-builtin`, etc.)—no inline styles. Colors are entirely controlled by a CSS theme file, making it trivial to swap themes without rebuilding.

## What It Gets Right That Syntect Doesn't

Let's see the difference with real code. Here's a function that uses lifetimes and turbofish:

### Before (syntect/regex-based)

With regex-based highlighters, this code gets mangled:

```text
fn parse<'a, T: FromStr>(&'a self) -> Result<T, T::Err> 
where
    T::Err: std::fmt::Debug,
{
    self.data.parse::<T>()?;
    let s: &'a str = self.get()?;
    Ok(s.parse()?)
}
```

Notice the problems:

- `'a` is split into a character literal `'` followed by identifier `a` — two different colors
- `.parse::<T>()` confuses the parser; the `<` and `>` look like comparison operators
- `self` gets highlighted as a regular identifier, nothing special

The regex doesn't understand that `'a` is a single semantic unit. It sees a quote character and panics.

### After (tree-sitter)

Now here's the same code with tree-sitter:

```rust
fn parse<'a, T: FromStr>(&'a self) -> Result<T, T::Err> 
where
    T::Err: std::fmt::Debug,
{
    self.data.parse::<T>()?;
    let s: &'a str = self.get()?;
    Ok(s.parse()?)
}
```

Tree-sitter correctly highlights:

- **Lifetimes**: `'a` as a single highlighted unit with its own distinct color
- **Turbofish**: `.parse::<T>()` with correct type highlighting inside the angle brackets
- **Macro invocations**: `format!()` would be scoped as `function.macro`
- **Impl blocks with where clauses**: types, traits, and keywords all correctly distinguished
- **`self`**: recognized as a builtin variable, highlighted distinctly from regular identifiers

The difference is night and day. Tree-sitter parses the full AST, so it knows that `'a` is a lifetime parameter, not a character literal. It understands that `::<T>` is the turbofish operator introducing a type argument, not a comparison followed by generics.

This matters more than aesthetics. Correct highlighting helps readers parse code at a glance. When lifetimes, types, and keywords are consistently colored, the structure of the code becomes immediately visible.

## Theme System

Taxus ships with two built-in SCSS partials: light (GitHub-inspired) and dark (Catppuccin-inspired). Users swap themes by changing a single `@import` line. Custom themes are just CSS mapping the `hl-*` classes—no tooling required. Both themes ship with `taxus init` so highlighting works out of the box.

The class naming follows a consistent pattern: `hl-keyword`, `hl-function`, `hl-type`, `hl-string`, `hl-comment`, `hl-operator`, and so on. This makes it easy to customize specific highlights without hunting through inscrutable class names.

## Configuration

Syntax highlighting is enabled by default—zero-config for the common case. It's configurable via `site.toml`:

```toml
[highlight]
enabled = true
class_prefix = "hl-"
```

When disabled, code blocks render as plain `<pre><code>`—no tree-sitter overhead. This is useful if you prefer client-side highlighting or want to minimize build times for content-heavy sites without much code.

## What's Next

The highlighting foundation sets up a future feature: interactive Rust playground islands. Static code blocks are SSR'd with tree-sitter at build time; a Yew WASM island can hydrate them into a runnable playground in the browser. This is only possible because Taxus already has an islands architecture—something no other Rust SSG offers.

Imagine a reader being able to modify the code snippet right in your blog post and see the output instantly. That's the vision.

## Try It

Ready to see it in action?

```bash
# Clone and build
git clone https://github.com/crusty-rustacean/taxus
cd taxus

# Initialize a new site
cargo run -- init my-site
cd my-site

# Start the development server
cargo run -- serve
```

Drop a Rust code block into any Markdown file and watch it come alive with proper highlighting. The difference is subtle for simple snippets, but becomes obvious when you start writing real Rust with lifetimes, generics, and macros.

Happy writing.
