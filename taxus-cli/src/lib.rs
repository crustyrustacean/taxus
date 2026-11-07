//! Shared implementation for the `taxus` product command.
//!
//! The command-line package owns product-only concerns such as site
//! scaffolding, the development server, telemetry, and command parsing. The
//! reusable build APIs live in [`taxus`].

pub mod cli;
pub mod commands;
pub mod error;
pub mod init;
pub mod serve;
mod serve_error;
pub mod telemetry;
