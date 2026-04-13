//! Tests for the serve module.
//!
//! These tests follow TDD principles - testing the expected behavior
//! before implementation.

use std::path::PathBuf;
use taxus_lib::serve::{
    ChangeType, DevServerConfig, ReloadEvent, WatchEvent, WebSocketMessage,
    inject_live_reload_script,
};

// =============================================================================
// Error Type Tests
// =============================================================================

mod error_tests {
    use std::path::PathBuf;
    use taxus_lib::serve::ServeError;

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
        assert!(message.contains("/some/path"));
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
}

// =============================================================================
// WebSocket Message Tests
// =============================================================================

mod websocket_tests {
    use super::*;
    use chrono::Utc;

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
        // Uses lowercase due to serde rename_all = "lowercase"
        assert!(json.contains("reload"));
    }

    #[test]
    fn test_websocket_message_error() {
        let message = WebSocketMessage::Error {
            message: "Build failed".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        // Uses lowercase due to serde rename_all = "lowercase"
        assert!(json.contains("error"));
        assert!(json.contains("Build failed"));
    }
}

// =============================================================================
// Change Type Tests
// =============================================================================

mod change_type_tests {
    use super::*;

    #[test]
    fn test_categorize_content_file() {
        let path = PathBuf::from("content/blog/my-post.md");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Content);
    }

    #[test]
    fn test_categorize_template_file() {
        let path = PathBuf::from("templates/base.html");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Template);
    }

    #[test]
    fn test_categorize_style_file() {
        let path = PathBuf::from("styles/main.scss");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Style);
    }

    #[test]
    fn test_categorize_static_file() {
        let path = PathBuf::from("static/images/logo.png");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Static);
    }

    #[test]
    fn test_categorize_config_file() {
        let path = PathBuf::from("site.toml");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Config);
    }

    #[test]
    fn test_categorize_nested_content() {
        let path = PathBuf::from("content/blog/2024/january/post.md");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Content);
    }

    #[test]
    fn test_categorize_unknown_file() {
        let path = PathBuf::from("README.md");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Unknown);
    }

    #[test]
    fn test_change_type_serialization() {
        let ct = ChangeType::Content;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"Content\"");
    }

    #[test]
    fn test_change_type_deserialization() {
        let json = "\"Template\"";
        let ct: ChangeType = serde_json::from_str(json).unwrap();
        assert_eq!(ct, ChangeType::Template);
    }
}

// =============================================================================
// Watch Event Tests
// =============================================================================

mod watch_event_tests {
    use super::*;

    #[test]
    fn test_watch_event_creation() {
        let paths = vec![
            PathBuf::from("content/post1.md"),
            PathBuf::from("content/post2.md"),
        ];
        let event = WatchEvent::new(ChangeType::Content, paths.clone());

        assert_eq!(event.change_type, ChangeType::Content);
        assert_eq!(event.paths, paths);
    }

    #[test]
    fn test_watch_event_empty_paths() {
        let event = WatchEvent::new(ChangeType::Config, vec![]);
        assert!(event.paths.is_empty());
    }
}

// =============================================================================
// HTML Injection Tests
// =============================================================================

mod injector_tests {
    use super::*;

    #[test]
    fn test_inject_before_body_end() {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
<h1>Hello</h1>
</body>
</html>"#;

        let result = inject_live_reload_script(html);
        assert!(result.contains("<script>"));
        assert!(result.contains("WebSocket"));
        assert!(result.contains("</script>"));
        // Script should be injected before </body>
        let body_end_pos = result.rfind("</body>").unwrap();
        let script_pos = result.rfind("<script>").unwrap();
        assert!(script_pos < body_end_pos);
    }

    #[test]
    fn test_inject_no_body_tag() {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
</html>"#;

        let result = inject_live_reload_script(html);
        // Should append at the end if no </body> tag
        assert!(result.contains("<script>"));
        // Script is appended after </html> since no </body> found
        assert!(result.contains("</html>"));
    }

    #[test]
    fn test_inject_empty_html() {
        let html = "";
        let result = inject_live_reload_script(html);
        // Should handle empty input gracefully
        assert!(result.contains("<script>"));
    }

    #[test]
    fn test_inject_already_has_script() {
        let html = r#"<html>
<body>
<script>const ws = new WebSocket('ws://localhost:3000/__ws__');</script>
</body>
</html>"#;

        let result = inject_live_reload_script(html);
        // Should still inject (we don't check for existing script)
        assert!(result.contains("taxus"));
    }

    #[test]
    fn test_live_reload_script_content() {
        // Verify the script contains expected elements
        assert!(taxus_lib::serve::LIVE_RELOAD_SCRIPT.contains("WebSocket"));
        assert!(taxus_lib::serve::LIVE_RELOAD_SCRIPT.contains("__ws__"));
        assert!(taxus_lib::serve::LIVE_RELOAD_SCRIPT.contains("location.reload"));
    }
}

// =============================================================================
// DevServerConfig Tests
// =============================================================================

mod config_tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DevServerConfig::default();

        assert_eq!(config.port, 3000);
        assert_eq!(config.output_dir, PathBuf::from("dist"));
        assert_eq!(config.site_dir, PathBuf::from("."));
    }

    #[test]
    fn test_config_with_custom_port() {
        let config = DevServerConfig::default().with_port(8080);

        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_config_with_custom_output_dir() {
        let config = DevServerConfig::default().with_output_dir(PathBuf::from("output"));

        assert_eq!(config.output_dir, PathBuf::from("output"));
    }

    #[test]
    fn test_config_with_custom_site_dir() {
        let config = DevServerConfig::default().with_site_dir(PathBuf::from("mysite"));

        assert_eq!(config.site_dir, PathBuf::from("mysite"));
    }
}
