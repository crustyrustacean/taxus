// taxus-generator/src/highlighting.rs

// module declarations

pub mod engine;
pub mod languages;
pub mod render;

// re-exports
pub use engine::{CodeHighlighter, HighlightResult};
pub use languages::LanguageRegistry;
