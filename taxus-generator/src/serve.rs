// taxus-generator/src/serve.rs

//! Development server with hot-reload support.
//!
//! This module provides a development server that serves static files and
//! automatically reloads the browser when content, templates, or styles change.
//!
//! # Overview
//!
//! - [`DevServer`]: The main development server
//! - [`DevServerConfig`]: Configuration options for the server
//! - [`ReloadEvent`]: Events sent to browsers for live reload
//!
//! # Example
//!
//! ```no_run
//! use taxus_lib::serve::{DevServer, DevServerConfig, RebuildFn};
//! use std::path::PathBuf;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DevServerConfig::default()
//!         .with_port(3000)
//!         .with_site_dir(PathBuf::from("."))
//!         .with_output_dir(PathBuf::from("dist"));
//!
//!     let rebuild: RebuildFn = Arc::new(|| Ok(()));
//!     let server = DevServer::new(config, rebuild);
//!     server.run().await?;
//!
//!     Ok(())
//! }
//! ```

mod coordinator;
mod error;
mod injector;
mod server;
mod watcher;
mod websocket;

pub use error::ServeError;
pub use injector::{LIVE_RELOAD_SCRIPT, inject_live_reload_script};
pub use server::{DevServer, DevServerConfig, RebuildFn};
pub use watcher::{ChangeType, FileWatcher, WatchEvent, WatcherGuard};
pub use websocket::{ReloadEvent, WebSocketMessage};
