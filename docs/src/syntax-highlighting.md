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

| Language | Identifier | Aliases |
|----------|------------|---------|
| Rust | `rust` | `rs` |

Additional languages can be added by enabling more tree-sitter grammar features.

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

1. Add the tree-sitter grammar to `Cargo.toml` as an optional dependency
2. Create a feature flag for the language
3. Add `LanguageSpec` registration in `languages.rs`
4. Add highlight queries in `queries/<lang>/highlights.scm`

Example for adding JavaScript support:

```toml
# Cargo.toml
[dependencies]
tree-sitter-javascript = { version = "0.20", optional = true }

[features]
lang-javascript = ["tree-sitter-javascript"]
```

```rust
// languages.rs
#[cfg(feature = "lang-javascript")]
fn register_javascript(&mut self) {
    let spec = LanguageSpec {
        name: "javascript",
        language: tree_sitter_javascript::LANGUAGE.into(),
        highlight_query: include_str!("queries/javascript/highlights.scm"),
        injection_query: None,
        locals_query: None,
    };
    self.register(spec, &["js", "javascript"]);
}
```
