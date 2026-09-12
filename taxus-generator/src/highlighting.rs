// taxus-generator/src/highlighting.rs

//! Syntax highlighting with tree-sitter (emit phase).
//!
//! Fenced code blocks are highlighted while Markdown is rendered in stage
//! 3. Grammars are registered per language behind Cargo features; see the
//! book's [Syntax Highlighting](https://crustyrustacean.github.io/taxus/syntax-highlighting.html) chapter.

// module declarations

pub mod engine;
pub mod languages;

// re-exports
pub use engine::{CodeHighlighter, HighlightResult};
pub use languages::LanguageRegistry;
