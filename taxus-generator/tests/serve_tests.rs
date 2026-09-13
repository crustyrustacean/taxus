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

// =============================================================================
// #11: component-based classification must not substring-match
// =============================================================================

mod change_type_regression_tests {
    use super::*;

    #[test]
    fn test_file_named_static_is_not_static_dir() {
        // content/static.md is a CONTENT file whose name contains "static".
        let path = PathBuf::from("content/static.md");
        assert_eq!(ChangeType::from_path(&path), ChangeType::Content);
    }

    #[test]
    fn test_dir_named_my_content_is_unknown() {
        // Substring matching would have called this Content.
        let path = PathBuf::from("my-content/notes.md");
        assert_eq!(ChangeType::from_path(&path), ChangeType::Unknown);
    }

    #[test]
    fn test_dir_named_styles_archive_is_unknown() {
        let path = PathBuf::from("styles-archive/old.scss");
        assert_eq!(ChangeType::from_path(&path), ChangeType::Unknown);
    }

    #[test]
    fn test_static_named_content_md_is_static() {
        let path = PathBuf::from("static/content.md");
        assert_eq!(ChangeType::from_path(&path), ChangeType::Static);
    }

    #[test]
    fn test_absolute_style_path() {
        // Classification is site-relative: the watcher strips the site
        // prefix before calling from_path. A raw absolute path's first
        // component is the filesystem root, so it classifies Unknown —
        // pinning that is the #48 contract.
        let path = PathBuf::from("/site/styles/main.scss");
        assert_eq!(ChangeType::from_path(&path), ChangeType::Unknown);
    }

    #[test]
    fn test_bare_directory_names_still_classify() {
        // Relative paths where the dir is the only segment before the file.
        assert_eq!(
            ChangeType::from_path(&PathBuf::from("content/x.md")),
            ChangeType::Content
        );
    }
}

// =============================================================================
// #42/#48: watcher scope and debounce
// =============================================================================

mod watcher_scope_tests {
    use super::*;
    use std::time::Duration;
    use taxus_lib::serve::FileWatcher;

    /// A site-shaped temp directory with the four source dirs and dist/.
    fn site_skeleton() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        for sub in ["content", "templates", "styles", "static", "dist"] {
            std::fs::create_dir(dir.path().join(sub)).unwrap();
        }
        std::fs::write(dir.path().join("site.toml"), "# site").unwrap();
        dir
    }

    /// watch_dirs lists exactly the existing source dirs, never dist/.
    #[test]
    fn test_watch_dirs_excludes_output_dir() {
        let dir = site_skeleton();
        let watcher = FileWatcher::new(dir.path().to_path_buf()).unwrap();

        let watched = watcher.watch_dirs();
        let mut names: Vec<String> = watched
            .iter()
            .map(|d| d.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        names.sort();

        assert_eq!(names, vec!["content", "static", "styles", "templates"]);
        assert!(!watched.iter().any(|d| d.ends_with("dist")));
    }

    /// A file write inside dist/ produces no watch event within a generous
    /// window, even though the write itself succeeds (#48: no loop).
    #[tokio::test]
    async fn test_output_dir_write_produces_no_rebuild_event() {
        let dir = site_skeleton();
        let mut watcher = FileWatcher::new(dir.path().to_path_buf()).unwrap();
        watcher.start().unwrap();

        // Write into the output directory: the pathological case — a
        // section named `templates` means dist/templates/... is written
        // on every build.
        std::fs::create_dir_all(dir.path().join("dist/templates")).unwrap();
        std::fs::write(dir.path().join("dist/templates/index.html"), b"<html>").unwrap();

        // Recv with a timeout comfortably longer than the debounce window.
        let none = tokio::time::timeout(Duration::from_millis(600), watcher.recv()).await;
        assert!(
            none.is_err(),
            "an output-dir write must not trigger a rebuild event"
        );
    }

    /// A burst of rapid writes to content/ coalesces into one event
    /// (#42: debounce).
    #[tokio::test]
    async fn test_rapid_content_writes_coalesce_into_one_event() {
        let dir = site_skeleton();
        let mut watcher = FileWatcher::new(dir.path().to_path_buf()).unwrap();
        watcher.start().unwrap();

        // Four rapid writes — an editor saving twice, or a tool writing
        // a file and its metadata.
        for i in 0..4 {
            std::fs::write(dir.path().join(format!("content/post-{i}.md")), b"body").unwrap();
        }

        let event = tokio::time::timeout(Duration::from_secs(2), watcher.recv())
            .await
            .expect("a debounced event should arrive")
            .expect("watcher alive");

        // All four writes folded into the one event.
        assert_eq!(event.paths.len(), 4, "paths: {:?}", event.paths);
        assert_eq!(event.change_type, ChangeType::Content);

        // …and nothing else follows within a generous window.
        let extra = tokio::time::timeout(Duration::from_millis(600), watcher.recv()).await;
        assert!(extra.is_err(), "a second event arrived for one burst");
    }
}

// =============================================================================
// #40: serve draft plumbing
// =============================================================================

mod serve_drafts_tests {
    use super::*;

    /// The server config records the include-drafts choice, and the
    /// builder chain applies it: wiring tested at the units the CLI
    /// composes (the CLI layer is a three-field pass-through).
    #[test]
    fn test_dev_server_config_with_include_drafts() {
        let config = DevServerConfig::default().with_include_drafts(true);
        assert!(config.include_drafts);

        let config = DevServerConfig::default();
        assert!(!config.include_drafts);
    }
}
