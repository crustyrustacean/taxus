//! Error types for the development server.
//!
//! This module provides error handling for the serve command.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during development server operation.
#[derive(Debug, Error)]
pub enum ServeError {
    /// The specified port is already in use.
    #[error("Port {port} is already in use. Try a different port with --port")]
    PortInUse {
        /// The port number that's in use
        port: u16,
    },

    /// The site configuration file was not found.
    #[error("Configuration file not found: {0}. Run 'yew-ssg init' to create a new site.")]
    ConfigNotFound(PathBuf),

    /// A build failed during the serve session.
    #[error("Build failed: {0}")]
    BuildFailed(String),

    /// An I/O error occurred.
    #[error("I/O error for {path}: {source}")]
    Io {
        /// The path related to the I/O error
        path: PathBuf,
        /// The underlying I/O error
        #[source]
        source: std::io::Error,
    },

    /// The file watcher failed to start.
    #[error("Failed to start file watcher: {0}")]
    WatcherFailed(String),

    /// WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Server shutdown error.
    #[error("Server error: {0}")]
    Server(String),
}

impl From<std::io::Error> for ServeError {
    fn from(error: std::io::Error) -> Self {
        ServeError::Io {
            path: PathBuf::new(),
            source: error,
        }
    }
}

impl From<notify::Error> for ServeError {
    fn from(error: notify::Error) -> Self {
        ServeError::WatcherFailed(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_in_use_error_display() {
        let error = ServeError::PortInUse { port: 3000 };
        let message = format!("{}", error);
        assert!(message.contains("3000"));
        assert!(message.contains("already in use"));
    }

    #[test]
    fn test_config_not_found_error() {
        let path = PathBuf::from("/some/path");
        let error = ServeError::ConfigNotFound(path.clone());
        let message = format!("{}", error);
        assert!(message.contains("Configuration file not found"));
        assert!(message.contains("yew-ssg init"));
    }

    #[test]
    fn test_build_failed_error() {
        let reason = "Template parsing failed";
        let error = ServeError::BuildFailed(reason.to_string());
        let message = format!("{}", error);
        assert!(message.contains(reason));
    }

    #[test]
    fn test_io_error() {
        let path = PathBuf::from("/some/file.txt");
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = ServeError::Io {
            path: path.clone(),
            source: io_error,
        };
        let message = format!("{}", error);
        assert!(message.contains("I/O error"));
        assert!(message.contains("file not found"));
    }

    #[test]
    fn test_watcher_failed_error() {
        let error = ServeError::WatcherFailed("permission denied".to_string());
        let message = format!("{}", error);
        assert!(message.contains("file watcher"));
        assert!(message.contains("permission denied"));
    }

    #[test]
    fn test_websocket_error() {
        let error = ServeError::WebSocket("connection reset".to_string());
        let message = format!("{}", error);
        assert!(message.contains("WebSocket"));
        assert!(message.contains("connection reset"));
    }

    #[test]
    fn test_server_error() {
        let error = ServeError::Server("bind failed".to_string());
        let message = format!("{}", error);
        assert!(message.contains("Server error"));
        assert!(message.contains("bind failed"));
    }
}
