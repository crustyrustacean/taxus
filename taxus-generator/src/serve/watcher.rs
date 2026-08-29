//! File system watcher for detecting changes.
//!
//! This module provides file watching functionality to detect changes
//! in content, templates, styles, static files, and configuration.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use super::error::ServeError;

/// The type of file that changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ChangeType {
    /// Content file (Markdown in content/).
    Content,
    /// Template file (HTML in templates/).
    Template,
    /// Style file (SCSS in styles/).
    Style,
    /// Static file (in static/).
    Static,
    /// Configuration file (site.toml).
    Config,
    /// Unknown file type.
    Unknown,
}

impl ChangeType {
    /// Determine the change type from a file path.
    ///
    /// Matches on path *components*, not substrings: `content/blog/...`
    /// is Content, but a file at `my-content/notes.md` or one inside a
    /// directory named `styles-archive/` must not misclassify (#11).
    /// Substring matching also couldn't distinguish `content/static.md`
    /// (a content file that happens to be named "static") from anything
    /// under `static/`.
    pub fn from_path(path: &Path) -> Self {
        // Check for config file first (exact match)
        if path.ends_with("site.toml") {
            return ChangeType::Config;
        }

        let mut components = path.components().peekable();
        // Tolerate absolute paths (site-dir prefix) and "." segments by
        // scanning for the FIRST recognized component. A recognized name
        // only classifies when it is a directory (has a further component);
        // a file merely *named* "content"/"static" is not a directory hit.
        while let Some(comp) = components.next() {
            let matched = match comp.as_os_str().to_str() {
                Some("content") => ChangeType::Content,
                Some("templates") => ChangeType::Template,
                Some("styles") => ChangeType::Style,
                Some("static") => ChangeType::Static,
                _ => continue,
            };
            if components.peek().is_some() {
                return matched;
            }
        }

        ChangeType::Unknown
    }
}

/// A file change event.
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// The type of change.
    pub change_type: ChangeType,
    /// The paths that changed.
    pub paths: Vec<PathBuf>,
}

impl WatchEvent {
    /// Create a new watch event.
    pub fn new(change_type: ChangeType, paths: Vec<PathBuf>) -> Self {
        Self { change_type, paths }
    }

    /// Create a watch event from a notify event.
    pub fn from_notify_event(event: &Event) -> Self {
        let paths: Vec<PathBuf> = event.paths.clone();

        // Determine the change type from the first path
        let change_type = paths
            .first()
            .map(|p| ChangeType::from_path(p.as_path()))
            .unwrap_or(ChangeType::Unknown);

        Self { change_type, paths }
    }

    /// Check if this event should trigger a rebuild.
    pub fn should_rebuild(&self) -> bool {
        matches!(
            self.change_type,
            ChangeType::Content | ChangeType::Template | ChangeType::Style | ChangeType::Config
        )
    }
}

/// File watcher for detecting changes.
pub struct FileWatcher {
    /// The site directory to watch.
    site_dir: PathBuf,
    /// The watcher instance.
    watcher: RecommendedWatcher,
    /// Event receiver channel.
    event_rx: mpsc::Receiver<WatchEvent>,
}

impl FileWatcher {
    /// Create a new file watcher.
    pub fn new(site_dir: PathBuf) -> Result<Self, ServeError> {
        let (tx, rx) = mpsc::channel(64);

        // Create the watcher with a callback that sends events to our channel
        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        // Skip non-modification events
                        if !matches!(
                            event.kind,
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                        ) {
                            return;
                        }

                        // Filter out temporary files and hidden files
                        let paths: Vec<PathBuf> = event
                            .paths
                            .iter()
                            .filter(|p| {
                                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                !name.starts_with('.')
                                    && !name.ends_with('~')
                                    && !name.ends_with(".swp")
                            })
                            .cloned()
                            .collect();

                        if paths.is_empty() {
                            return;
                        }

                        let watch_event = WatchEvent::from_notify_event(&Event { paths, ..event });

                        // Only send if it should trigger a rebuild
                        if watch_event.should_rebuild() {
                            debug!("File change detected: {:?}", watch_event);
                            if tx.blocking_send(watch_event).is_err() {
                                error!("Failed to send watch event - receiver dropped");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {}", e);
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )
        .map_err(|e| ServeError::WatcherFailed(e.to_string()))?;

        Ok(Self {
            site_dir,
            watcher,
            event_rx: rx,
        })
    }

    /// Start watching the site directory.
    pub fn start(&mut self) -> Result<(), ServeError> {
        // Watch the site directory recursively
        self.watcher
            .watch(&self.site_dir, RecursiveMode::Recursive)
            .map_err(|e| ServeError::WatcherFailed(e.to_string()))?;

        info!("Watching for changes in: {}", self.site_dir.display());
        Ok(())
    }

    /// Get the directories being watched.
    pub fn watch_dirs(&self) -> Vec<PathBuf> {
        let dirs = vec![
            self.site_dir.join("content"),
            self.site_dir.join("templates"),
            self.site_dir.join("styles"),
            self.site_dir.join("static"),
        ];

        dirs.into_iter().filter(|d| d.exists()).collect()
    }

    /// Get the config file path.
    pub fn config_path(&self) -> PathBuf {
        self.site_dir.join("site.toml")
    }

    /// Receive the next watch event.
    pub async fn recv(&mut self) -> Option<WatchEvent> {
        self.event_rx.recv().await
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_watch_event_should_rebuild() {
        let content_event =
            WatchEvent::new(ChangeType::Content, vec![PathBuf::from("content/a.md")]);
        assert!(content_event.should_rebuild());

        let template_event = WatchEvent::new(
            ChangeType::Template,
            vec![PathBuf::from("templates/a.html")],
        );
        assert!(template_event.should_rebuild());

        let style_event = WatchEvent::new(ChangeType::Style, vec![PathBuf::from("styles/a.scss")]);
        assert!(style_event.should_rebuild());

        let config_event = WatchEvent::new(ChangeType::Config, vec![PathBuf::from("site.toml")]);
        assert!(config_event.should_rebuild());

        let static_event =
            WatchEvent::new(ChangeType::Static, vec![PathBuf::from("static/img.png")]);
        assert!(!static_event.should_rebuild());

        let unknown_event = WatchEvent::new(ChangeType::Unknown, vec![PathBuf::from("README.md")]);
        assert!(!unknown_event.should_rebuild());
    }
}
