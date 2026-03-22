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
//! use yew_ssg_lib::serve::{DevServer, DevServerConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DevServerConfig::default()
//!         .with_port(3000)
//!         .with_site_dir(PathBuf::from("."))
//!         .with_output_dir(PathBuf::from("dist"));
//!
//!     let server = DevServer::new(config);
//!     server.run().await?;
//!
//!     Ok(())
//! }
//! ```

mod error;
mod injector;
mod server;
mod watcher;
mod websocket;

pub use error::ServeError;
pub use injector::{LIVE_RELOAD_SCRIPT, inject_live_reload_script};
pub use server::{DevServer, DevServerConfig};
pub use watcher::{ChangeType, FileWatcher, WatchEvent};
pub use websocket::{ReloadEvent, WebSocketMessage};
