//! File system watcher for detecting changes.
//!
//! This module provides file watching functionality to detect changes
//! in content, templates, styles, static files, and configuration.
//!
//! ## Watch scope (#48)
//!
//! Only the source directories (`content/`, `templates/`, `styles/`,
//! `static/`) and `site.toml` are watched — never the site directory
//! recursively. A recursive watch would include the output directory,
//! so every build's own writes would be classified as source changes
//! and trigger another build: an infinite rebuild loop for any site
//! with a section named like a source directory (`content/templates/`
//! exists as a section in the wild).
//!
//! ## Debounce (#42)
//!
//! Editors emit 2–4 events per save (write, rename, metadata) and some
//! tools emit bursts. Raw events are coalesced in a 150 ms window: the
//! first event opens the window, every further event folds into it, and
//! one coalesced [`WatchEvent`] is sent when it closes. Without this,
//! each save queues one full rebuild per raw event. The coordinator
//! additionally folds events that queue up *during* a build; the
//! debounce exists so an ordinary save produces one event, not a burst.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use super::error::ServeError;

/// How long to wait after the first raw event before emitting one
/// coalesced [`WatchEvent`].
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(150);

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
    /// Determine the change type from a *site-relative* path.
    ///
    /// The first component must name a source directory (`content`,
    /// `templates`, `styles`, `static`) and have something under it;
    /// matching on path *components*, not substrings, so
    /// `my-content/notes.md` or `styles-archive/old.scss` never
    /// misclassify (#11), and a path under any other first component —
    /// `dist/templates/index.html` above all — is `Unknown` (#48).
    ///
    /// Callers holding absolute paths must strip the site-directory
    /// prefix first ([`FileWatcher`] does); an absolute path's first
    /// component is the filesystem root, not a source directory.
    pub fn from_path(path: &Path) -> Self {
        // Check for config file first (exact match)
        if path.ends_with("site.toml") {
            return ChangeType::Config;
        }

        let mut components = path.components().peekable();
        // Skip a leading "." (relative paths like "./content/a.md").
        if components.peek() == Some(&std::path::Component::CurDir) {
            components.next();
        }

        let matched = match components.next().map(|c| c.as_os_str().to_str()) {
            Some(Some("content")) => ChangeType::Content,
            Some(Some("templates")) => ChangeType::Template,
            Some(Some("styles")) => ChangeType::Style,
            Some(Some("static")) => ChangeType::Static,
            _ => return ChangeType::Unknown,
        };

        // A recognized name only classifies when it is a directory (has a
        // further component); a file merely *named* "content"/"static" at
        // the site root is not a directory hit.
        if components.peek().is_some() {
            matched
        } else {
            ChangeType::Unknown
        }
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
    ///
    /// `site_dir` is stripped from each path so classification sees
    /// site-relative paths ([`ChangeType::from_path`]).
    pub fn from_notify_event(site_dir: &Path, event: &Event) -> Self {
        let paths: Vec<PathBuf> = event
            .paths
            .iter()
            .map(|p| p.strip_prefix(site_dir).unwrap_or(p).to_path_buf())
            .collect();

        // Determine the change type from the first path
        let change_type = paths
            .first()
            .map(|p| ChangeType::from_path(p.as_path()))
            .unwrap_or(ChangeType::Unknown);

        Self { change_type, paths }
    }

    /// Check if this event should trigger a rebuild.
    ///
    /// Static files rebuild too (#42): the build's asset stage copies
    /// `static/` into the output, so a change must be picked up for the
    /// dev server to serve the new file.
    pub fn should_rebuild(&self) -> bool {
        matches!(
            self.change_type,
            ChangeType::Content
                | ChangeType::Template
                | ChangeType::Style
                | ChangeType::Static
                | ChangeType::Config
        )
    }
}

/// Coalesces raw notify events within [`DEBOUNCE_WINDOW`] into one
/// `WatchEvent` on the shared channel.
///
/// The notify callback is synchronous (it fires on the watcher's own
/// thread), so the window is a `Mutex<Option<...>>` shared between the
/// callback and a timer thread: the first event in a quiet period opens
/// the window and starts the timer; later events just fold in; when the
/// timer fires the accumulated event is sent and the window closes.
#[derive(Debug, Default)]
struct Debouncer {
    /// The event being accumulated, with the instant its window closes.
    pending: Option<(WatchEvent, Instant)>,
}

impl Debouncer {
    fn fold(&mut self, event: WatchEvent, deadline: Instant) {
        match &mut self.pending {
            Some((acc, _)) => {
                acc.paths.extend(event.paths);
                acc.paths.sort();
                acc.paths.dedup();
            }
            None => self.pending = Some((event, deadline)),
        }
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
        let debouncer = Arc::new(Mutex::new(Debouncer::default()));
        let site_dir_for_cb = site_dir.clone();
        // The timer thread waits out the debounce window and flushes the
        // accumulated event. One thread lives for the watcher's lifetime.
        {
            let debouncer = Arc::clone(&debouncer);
            std::thread::Builder::new()
                .name("taxus-watch-debounce".into())
                .spawn(move || {
                    loop {
                        // Sleep out the remaining window; a fresh window may
                        // have opened in the meantime, so re-check.
                        std::thread::sleep(DEBOUNCE_WINDOW);
                        let mut guard = debouncer.lock().unwrap();
                        let Some((_, deadline)) = guard.pending.as_ref() else {
                            continue;
                        };
                        let now = Instant::now();
                        if *deadline <= now {
                            let (event, _) = guard.pending.take().expect("checked above");
                            drop(guard);
                            debug!("Debounced watch event: {:?}", event);
                            if event.should_rebuild() && tx.blocking_send(event).is_err() {
                                error!("Failed to send watch event - receiver dropped");
                            }
                        }
                    }
                })
                .map_err(|e| ServeError::WatcherFailed(e.to_string()))?;
        }

        // Create the watcher with a callback that folds events into the
        // debounce window.
        let cb_debouncer = Arc::clone(&debouncer);
        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let site_dir = &site_dir_for_cb;
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

                        let watch_event =
                            WatchEvent::from_notify_event(site_dir, &Event { paths, ..event });

                        if watch_event.should_rebuild() {
                            let mut guard = cb_debouncer.lock().unwrap();
                            guard.fold(watch_event, Instant::now() + DEBOUNCE_WINDOW);
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {}", e);
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| ServeError::WatcherFailed(e.to_string()))?;

        Ok(Self {
            site_dir,
            watcher,
            event_rx: rx,
        })
    }

    /// Start watching the site's sources.
    ///
    /// Watches only the source directories that exist plus `site.toml`
    /// (#48) — never the site directory recursively, which would include
    /// the output directory and rebuild in a loop.
    pub fn start(&mut self) -> Result<(), ServeError> {
        let mut watched = 0;
        for dir in self.watch_dirs() {
            self.watcher
                .watch(&dir, RecursiveMode::Recursive)
                .map_err(|e| ServeError::WatcherFailed(e.to_string()))?;
            watched += 1;
        }

        // The config file is watched as a single file, not its directory:
        // a directory watch on the site root is exactly what #48 forbids.
        let config = self.config_path();
        if config.exists() {
            if let Err(e) = self.watcher.watch(&config, RecursiveMode::NonRecursive) {
                // Some platforms cannot watch single files; the site.toml
                // sits in the site root, which we deliberately do not
                // watch as a directory. Log and continue rather than
                // failing the server over an optional convenience.
                tracing::warn!(
                    error = %e,
                    "Cannot watch {} for changes; edits to it will not rebuild",
                    config.display()
                );
            } else {
                watched += 1;
            }
        }

        info!(
            site_dir = %self.site_dir.display(),
            watches = watched,
            "Watching site sources for changes"
        );
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

    /// Split the watcher into a [`WatcherGuard`] and the raw event receiver.
    ///
    /// The guard keeps the OS watcher registered; drop it to stop watching.
    /// The receiver yields the same events [`FileWatcher::recv`] would, but
    /// as a plain channel so a consumer can also `try_recv` to drain events
    /// that queued up while it was busy.
    pub fn into_receiver(self) -> (WatcherGuard, mpsc::Receiver<WatchEvent>) {
        (
            WatcherGuard {
                _watcher: self.watcher,
            },
            self.event_rx,
        )
    }
}

/// Keeps the underlying OS watcher alive after [`FileWatcher::into_receiver`].
///
/// Dropping the guard unregisters the watcher and closes the event channel.
pub struct WatcherGuard {
    _watcher: RecommendedWatcher,
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

        // #42: static files are part of the build output; a change must
        // rebuild so the copied asset is refreshed.
        let static_event =
            WatchEvent::new(ChangeType::Static, vec![PathBuf::from("static/img.png")]);
        assert!(static_event.should_rebuild());

        let unknown_event = WatchEvent::new(ChangeType::Unknown, vec![PathBuf::from("README.md")]);
        assert!(!unknown_event.should_rebuild());
    }

    /// #48: an event under the output directory must never classify as a
    /// source change. With the narrowed watch scope `dist/` is not watched
    /// at all; this test pins the classification contract regardless — if
    /// scope ever widens again, an output write must stay inert.
    #[test]
    fn test_output_dir_event_does_not_classify_as_template() {
        let path = PathBuf::from("dist/templates/index.html");
        assert_ne!(ChangeType::from_path(&path), ChangeType::Template);
    }

    #[test]
    fn test_output_dir_event_does_not_classify_as_content() {
        let path = PathBuf::from("dist/content/post/index.html");
        assert_ne!(ChangeType::from_path(&path), ChangeType::Content);
    }

    /// #42: events within one debounce window fold into a single event
    /// with deduplicated paths.
    #[test]
    fn test_debouncer_folds_events_in_one_window() {
        let mut d = Debouncer::default();
        let deadline = Instant::now() + DEBOUNCE_WINDOW;

        d.fold(
            WatchEvent::new(ChangeType::Content, vec![PathBuf::from("content/a.md")]),
            deadline,
        );
        d.fold(
            WatchEvent::new(
                ChangeType::Content,
                vec![PathBuf::from("content/a.md"), PathBuf::from("content/b.md")],
            ),
            deadline + Duration::from_millis(10),
        );

        let (event, _) = d.pending.take().unwrap();
        assert_eq!(event.change_type, ChangeType::Content);
        assert_eq!(
            event.paths,
            vec![PathBuf::from("content/a.md"), PathBuf::from("content/b.md")]
        );
    }

    /// A second burst after the window flushed starts fresh.
    #[test]
    fn test_debouncer_window_resets_after_flush() {
        let mut d = Debouncer::default();
        let deadline = Instant::now();
        d.fold(
            WatchEvent::new(ChangeType::Content, vec![PathBuf::from("content/a.md")]),
            deadline,
        );
        let _ = d.pending.take();

        d.fold(
            WatchEvent::new(
                ChangeType::Template,
                vec![PathBuf::from("templates/x.html")],
            ),
            deadline,
        );
        let (event, _) = d.pending.take().unwrap();
        assert_eq!(event.change_type, ChangeType::Template);
        assert_eq!(event.paths, vec![PathBuf::from("templates/x.html")]);
    }
}
