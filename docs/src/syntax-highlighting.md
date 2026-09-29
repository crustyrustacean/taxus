# Syntax Highlighting

Taxus provides syntax highlighting for code blocks using [tree-sitter](https://tree-sitter.github.io/tree-sitter/), a fast and accurate parsing library.

## Overview

Code blocks in Markdown are automatically highlighted during the build process. Tree-sitter provides:

- **Accurate parsing**: Uses real language grammars, not regex patterns
- **Fast performance**: Incremental parsing for quick builds
- **Rich highlighting**: Detailed semantic token classification

## Usage

Add a language identifier to your fenced code blocks:

````markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
````

This renders with syntax highlighting:

```rust
fn main() {
    println!("Hello, world!");
}
```

## Supported Languages

Languages are enabled via Cargo features when building Taxus:

| Language | Identifier | Aliases | Feature |
|----------|------------|---------|---------|
| Rust | `rust` | `rs` | `lang-rust` (default) |
| JavaScript | `javascript` | `js`, `mjs`, `cjs` | `lang-javascript` (default) |
| CSS | `css` | — | `lang-css` (default) |
| HTML | `html` | `htm` | `lang-html` (default) |

### Embedded languages in HTML

An `html` code block highlights its markup *and* the languages inside
it — a `<script>` body is parsed as JavaScript, a `<style>` body and a
`style="…"` attribute as CSS:

```html
<button class="counter" onclick="bump()">0</button>
<script>
  function bump() { count += 1; }
</script>
```

This is an *injection*: the markup is parsed as HTML, and the embedded
bodies are handed to their own grammars, exactly the mechanism Rust's
`macro_rules!` highlighting uses. A language that is not registered is
simply not injected, so an `html` block on a site built with
`--no-default-features --features lang-rust` still highlights its tags
but leaves the script body as plain text.

One consequence worth knowing: a block mixing markup with a
*template* language (for example `<h1>{{ page.title }}</h1>` in a Tera
template) is not valid HTML, so it falls back to plain, unstyled text.
Label such blocks for the language they actually contain.

### Feature flags

All four grammars are enabled by default. To build without some:

```bash
cargo build --release --no-default-features --features lang-rust,webp-lossy
```

## Configuration

Configure syntax highlighting in `site.toml`:

```toml
[highlight]
enabled = true
class_prefix = "hl-"
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable or disable syntax highlighting |
| `class_prefix` | string | `"hl-"` | CSS class prefix for highlight spans |

### Disabling Highlighting

To disable highlighting globally:

```toml
[highlight]
enabled = false
```

Code blocks will still render, but without syntax highlighting spans.

### Custom Class Prefix

Use a custom prefix for CSS classes:

```toml
[highlight]
class_prefix = "syntax-"
```

This generates classes like `syntax-keyword`, `syntax-string`, etc.

## Highlight Classes

Taxus generates semantic CSS classes for each token type:

| Class | Description |
|-------|-------------|
| `hl-keyword` | Keywords (fn, let, struct, impl, etc.) |
| `hl-string` | String literals |
| `hl-string-special` | Special strings (raw strings, format strings) |
| `hl-comment` | Comments |
| `hl-function` | Function names |
| `hl-function-builtin` | Built-in functions |
| `hl-function-macro` | Macro invocations |
| `hl-type` | Type names |
| `hl-type-builtin` | Built-in types (u32, str, etc.) |
| `hl-constant` | Constants |
| `hl-constant-builtin` | Built-in constants |
| `hl-number` | Numeric literals |
| `hl-constructor` | Constructors (Some, Ok, Err, etc.) |
| `hl-variable` | Variables |
| `hl-variable-builtin` | Built-in variables (self, Self) |
| `hl-variable-parameter` | Function parameters |
| `hl-property` | Struct fields/properties |
| `hl-label` | Lifetimes and labels |
| `hl-attribute` | Attributes (#[derive], #[cfg], etc.) |
| `hl-operator` | Operators (=, +, -, etc.) |
| `hl-punctuation` | General punctuation |
| `hl-punctuation-bracket` | Brackets and braces |
| `hl-punctuation-delimiter` | Commas, semicolons |
| `hl-tag` | HTML/XML tags |

## Styling

### Built-in Themes

Taxus includes two highlight themes:

- **Light theme**: `_highlight-light.scss` — GitHub-inspired colors
- **Dark theme**: `_highlight-dark.scss` — Catppuccin-inspired colors

Import in your main stylesheet:

```scss
// Light theme (default)
@use "highlight-light";

// Or dark theme
@use "highlight-dark";
```

### Custom Themes

Create custom themes by styling the highlight classes:

```scss
// Custom syntax highlighting theme
.hl-keyword { color: #ff79c6; }
.hl-string { color: #f1fa8c; }
.hl-comment { color: #6272a4; font-style: italic; }
.hl-function { color: #50fa7b; }
.hl-type { color: #8be9fd; }
.hl-number { color: #bd93f9; }
```

### Base Styles

Include base styles for code blocks:

```scss
pre.highlight {
  background-color: #f6f8fa;
  border: 1px solid #e1e4e8;
  border-radius: 6px;
  padding: 16px;
  overflow-x: auto;
  font-size: 0.875rem;
  line-height: 1.45;

  code {
    background: none;
    padding: 0;
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  }
}
```

## Unsupported Languages

When a language is not supported, the code block renders as plain text with HTML escaping:

````markdown
```brainfuck
++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>
```
````

The code will display in a code block without syntax highlighting, but HTML special characters are properly escaped.

## Adding New Languages

To add support for additional languages:

1. Add the tree-sitter grammar to `taxus-generator/Cargo.toml` as an optional dependency
2. Create a feature flag for the language
3. Add `LanguageSpec` registration in `taxus-generator/src/highlighting/languages.rs`
4. Add highlight queries in `taxus-generator/src/highlighting/queries/<lang>/highlights.scm`

Example for adding JavaScript support:

```toml
# Cargo.toml
[dependencies]
tree-sitter-json = { version = "0.24", optional = true }

[features]
lang-json = ["dep:tree-sitter-json"]

# ...and add it to `default` if it should ship enabled
```

```rust
// languages.rs
#[cfg(feature = "lang-json")]
fn register_json(&mut self) {
    let spec = LanguageSpec {
        name: "json",
        language: tree_sitter_json::LANGUAGE.into(),
        highlight_query: include_str!("queries/json/highlights.scm"),
        injection_query: None,
        locals_query: None,
    };
    self.register(spec, &[]);
}
```

And call `registry.register_json();` from `LanguageRegistry::new`.

Three things the web-language PR made concrete:

- **Node names are not guessable.** A capture that is not a real node
  type in the grammar is a *query* error, and `CodeHighlighter::new`
  turns that into a panic. `queries::every_registered_query_compiles`
  in `languages.rs` now guards this for every registered language —
  add a query there and you get the same guard for free.
- **Capture names are a closed set.** Only the names in
  `engine::HIGHLIGHT_NAMES` are accepted; anything else fails at
  `configure`. Reusing the shared vocabulary also means the shipped
  light/dark themes style the new language with no CSS changes.
- **`injection_query` needs the target grammar registered.** An
  injection names a language with `#set! injection.language "..."`,
  and an unregistered name resolves to nothing — the injected range is
  left unhighlighted rather than failing the build.
