// Common library code

//! Code shared by the generator (build-time rendering) and the WASM client
//! (browser hydration): the island components and the search index format.
//!
//! An island is a Yew component rendered to HTML at build time by the
//! generator's `island()` template function and hydrated in the browser by
//! `taxus-client`. Both sides compile this crate, so a component has one
//! source.

// module declarations
pub mod components;
pub mod hooks;
pub mod search;

// re-exports
pub use search::*;
