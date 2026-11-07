//! WebSocket message types for live reload.
//!
//! This module defines the messages sent between the server and browser
//! via WebSocket for live reload functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::watcher::ChangeType;

/// A reload event sent to connected browsers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadEvent {
    /// The type of change that triggered the reload.
    pub change_type: ChangeType,
    /// When the change occurred.
    pub timestamp: DateTime<Utc>,
    /// The files that changed.
    pub files: Vec<String>,
}

impl ReloadEvent {
    /// Create a new reload event.
    pub fn new(change_type: ChangeType, files: Vec<String>) -> Self {
        Self {
            change_type,
            timestamp: Utc::now(),
            files,
        }
    }
}

/// Messages sent over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WebSocketMessage {
    /// A reload request sent to the browser.
    Reload(ReloadEvent),

    /// An error occurred during build.
    Error {
        /// The error message.
        message: String,
    },

    /// Connected confirmation.
    Connected {
        /// Server version or identifier.
        server: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reload_event_serialization() {
        let event = ReloadEvent {
            change_type: ChangeType::Content,
            timestamp: Utc::now(),
            files: vec!["content/blog/post.md".to_string()],
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Content"));
        assert!(json.contains("content/blog/post.md"));
    }

    #[test]
    fn test_reload_event_deserialization() {
        let json = r#"{
            "change_type": "Template",
            "timestamp": "2024-01-15T10:30:00Z",
            "files": ["templates/base.html", "templates/page.html"]
        }"#;

        let event: ReloadEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.change_type, ChangeType::Template);
        assert_eq!(event.files.len(), 2);
    }

    #[test]
    fn test_websocket_message_reload() {
        let event = ReloadEvent {
            change_type: ChangeType::Style,
            timestamp: Utc::now(),
            files: vec!["styles/main.scss".to_string()],
        };
        let message = WebSocketMessage::Reload(event);

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("reload"));
    }

    #[test]
    fn test_websocket_message_error() {
        let message = WebSocketMessage::Error {
            message: "Build failed".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Build failed"));
    }

    #[test]
    fn test_websocket_message_connected() {
        let message = WebSocketMessage::Connected {
            server: "taxus-dev".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("taxus-dev"));
    }
}
