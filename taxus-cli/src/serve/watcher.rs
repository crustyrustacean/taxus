//! File system watcher for detecting changes.
//!
//! This module provides file watching functionality to detect changes
//! in content, templates, styles, static files, and configuration.
//!
//! ## Watch scope (#48)
//!
//! Only the configured source directories (the defaults are `content/`,
//! `templates/`, `styles/`, `static/`, and `scripts/`) and root config files
//! are watched — never the site directory recursively. A recursive watch would include the output directory,
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

use crate::serve_error::ServeError;

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
    /// TypeScript source (in scripts/).
    Script,
    /// Configuration file (site.toml).
    Config,
    /// Unknown file type.
    Unknown,
}

impl ChangeType {
    /// Determine the change type from a *site-relative* path.
    ///
    /// For the default layout, the first component must name a source
    /// directory (`content`, `templates`, `styles`, `static`, `scripts`) and
    /// have something under it; matching on path *components*, not substrings, so
    /// `my-content/notes.md` or `styles-archive/old.scss` never
    /// misclassify (#11), and a path under any other first component —
    /// `dist/templates/index.html` above all — is `Unknown` (#48).
    ///
    /// Callers holding absolute paths must strip the site-directory
    /// prefix first ([`FileWatcher`] does); an absolute path's first
    /// component is the filesystem root, not a source directory.
    pub fn from_path(path: &Path) -> Self {
        // Config files live at the site root, not under a watched source dir.
        if path == Path::new("site.toml") || path == Path::new("tsconfig.json") {
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
            Some(Some("scripts")) => ChangeType::Script,
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

/// The site-relative form of a path notify reported.
///
/// notify emits absolute paths. When `site_dir` is absolute — the
/// invariant [`FileWatcher::new`] establishes by canonicalizing before
/// any event arrives — a plain prefix strip is exact. When a caller
/// passes a relative or differently-spelled `site_dir`, fall back to
/// locating the site directory by name and taking everything after it,
/// or, for a name-less `site_dir` like `.`, cut at the first
/// source-directory component. A path matching neither is returned
/// unchanged, which classifies `Unknown` and is dropped: inert, never
/// misattributed to some other source directory.
fn strip_site_prefix(path: &Path, site_dir: &Path) -> PathBuf {
    if let Ok(rel) = path.strip_prefix(site_dir) {
        return rel.to_path_buf();
    }

    let comps: Vec<_> = path.components().collect();
    let source_dirs = ["content", "templates", "styles", "static", "scripts"];

    if let Some(site_name) = site_dir.file_name() {
        if let Some(i) = comps.iter().rposition(|c| c.as_os_str() == site_name)
            && i + 1 < comps.len()
        {
            return comps[i + 1..].iter().collect();
        }
        // A named site_dir that does not appear in this path at all:
        // the path is not ours.
        return path.to_path_buf();
    }

    // Name-less site_dir (`.`, `..`, a filesystem root): the watch roots
    // are the authority — cut at the first source-directory component.
    if let Some(i) = comps
        .iter()
        .position(|c| source_dirs.contains(&c.as_os_str().to_str().unwrap_or("")))
    {
        return comps[i..].iter().collect();
    }

    path.to_path_buf()
}

/// Classify an event path against configured source roots. The longest
/// matching root wins so nested configured directories remain deterministic.
fn classify_path(
    absolute: &Path,
    relative: &Path,
    source_dirs: &[(PathBuf, ChangeType)],
) -> ChangeType {
    if relative == Path::new("site.toml") || relative == Path::new("tsconfig.json") {
        return ChangeType::Config;
    }

    if source_dirs.is_empty() {
        return ChangeType::from_path(relative);
    }

    source_dirs
        .iter()
        .filter_map(|(root, change_type)| {
            let remainder = absolute.strip_prefix(root).ok()?;
            (!remainder.as_os_str().is_empty()).then_some((root.components().count(), *change_type))
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, change_type)| change_type)
        .unwrap_or(ChangeType::Unknown)
}

impl WatchEvent {
    /// Create a new watch event.
    pub fn new(change_type: ChangeType, paths: Vec<PathBuf>) -> Self {
        Self { change_type, paths }
    }

    /// Create a watch event from a notify event using the default source
    /// directory names.
    pub fn from_notify_event(site_dir: &Path, event: &Event) -> Self {
        Self::from_notify_event_with_source_dirs(site_dir, event, &[])
    }

    /// Create a watch event using configured source roots.
    ///
    /// Notify reports absolute paths. The site prefix is stripped from the
    /// paths exposed in the event, while classification uses the configured
    /// absolute roots. This keeps custom directories such as `src/posts`
    /// classified as [`ChangeType::Content`] without relying on their names.
    pub fn from_notify_event_with_source_dirs(
        site_dir: &Path,
        event: &Event,
        source_dirs: &[(PathBuf, ChangeType)],
    ) -> Self {
        let paths: Vec<PathBuf> = event
            .paths
            .iter()
            .map(|p| strip_site_prefix(p, site_dir))
            .collect();

        let change_type = event
            .paths
            .iter()
            .zip(paths.iter())
            .map(|(absolute, relative)| classify_path(absolute, relative, source_dirs))
            .find(|change_type| *change_type != ChangeType::Unknown)
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
                | ChangeType::Script
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
    /// Configured source directories and their event classifications.
    source_dirs: Vec<(PathBuf, ChangeType)>,
    /// The watcher instance.
    watcher: RecommendedWatcher,
    /// Event receiver channel.
    event_rx: mpsc::Receiver<WatchEvent>,
}

impl FileWatcher {
    /// Create a new file watcher.
    ///
    /// `site_dir` is canonicalized here. Two things depend on it: the
    /// watch roots (where a relative path works) and the strip in
    /// [`WatchEvent::from_notify_event`], which must remove the site
    /// prefix from the ABSOLUTE paths notify reports. With a relative
    /// `site_dir` — the CLI default is `.` — that strip silently
    /// failed, every event classified `Unknown`, and hot reload did
    /// nothing. Canonicalizing once fixes both callers.
    pub fn new(site_dir: PathBuf) -> Result<Self, ServeError> {
        let source_dirs = [
            (site_dir.join("content"), ChangeType::Content),
            (site_dir.join("templates"), ChangeType::Template),
            (site_dir.join("styles"), ChangeType::Style),
            (site_dir.join("static"), ChangeType::Static),
            (site_dir.join("scripts"), ChangeType::Script),
        ]
        .into_iter()
        .collect();
        Self::new_with_source_dirs(site_dir, source_dirs)
    }

    /// Create a watcher with resolved source directories and their semantic
    /// change types. Paths are canonicalized when they exist so comparisons
    /// remain stable on platforms such as Windows.
    pub fn new_with_source_dirs(
        site_dir: PathBuf,
        source_dirs: Vec<(PathBuf, ChangeType)>,
    ) -> Result<Self, ServeError> {
        // `canonicalize` needs the directory to exist; `serve` has
        // already read site.toml from it, so it does. `dunce`-style
        // Windows paths are avoided by `canonicalize` normalising the
        // UNC prefix.
        let site_dir = std::fs::canonicalize(&site_dir).map_err(|e| {
            ServeError::WatcherFailed(format!(
                "Cannot resolve site directory {}: {e}",
                site_dir.display()
            ))
        })?;
        let source_dirs = source_dirs
            .into_iter()
            .map(|(path, change_type)| {
                let path = std::fs::canonicalize(&path).unwrap_or(path);
                (path, change_type)
            })
            .collect::<Vec<_>>();

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
        let source_dirs_for_cb = source_dirs.clone();
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

                        let watch_event = WatchEvent::from_notify_event_with_source_dirs(
                            site_dir,
                            &Event { paths, ..event },
                            &source_dirs_for_cb,
                        );

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
            source_dirs,
            watcher,
            event_rx: rx,
        })
    }

    /// Start watching the site's sources.
    ///
    /// Watches only the configured source directories that exist plus the
    /// root config files (#48) — never the site directory recursively, which
    /// would include the output directory and rebuild in a loop.
    pub fn start(&mut self) -> Result<(), ServeError> {
        let mut watched = 0;
        for dir in self.watch_dirs() {
            self.watcher
                .watch(&dir, RecursiveMode::Recursive)
                .map_err(|e| ServeError::WatcherFailed(e.to_string()))?;
            watched += 1;
        }

        // Root config files are watched individually, never by watching the
        // site root (which would include output and cause rebuild loops).
        for config in self.config_paths().into_iter().filter(|path| path.exists()) {
            if let Err(e) = self.watcher.watch(&config, RecursiveMode::NonRecursive) {
                // Some platforms cannot watch single files. Log and continue
                // rather than failing the server over an optional convenience.
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

    /// Get the configured source directories that exist and are being watched.
    pub fn watch_dirs(&self) -> Vec<PathBuf> {
        self.source_dirs
            .iter()
            .map(|(path, _)| path)
            .filter(|path| path.exists())
            .cloned()
            .collect()
    }

    /// Get the site configuration file path.
    pub fn config_path(&self) -> PathBuf {
        self.site_dir.join("site.toml")
    }

    /// Root-level build configuration files watched individually.
    pub fn config_paths(&self) -> [PathBuf; 2] {
        [self.config_path(), self.site_dir.join("tsconfig.json")]
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
    fn test_categorize_typescript_source() {
        let path = PathBuf::from("scripts/main.ts");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Script);
        assert!(WatchEvent::new(change_type, vec![path]).should_rebuild());
    }

    #[test]
    fn test_typescript_config_is_a_root_config_file() {
        assert_eq!(
            ChangeType::from_path(Path::new("tsconfig.json")),
            ChangeType::Config
        );
        assert_eq!(
            ChangeType::from_path(Path::new("nested/tsconfig.json")),
            ChangeType::Unknown
        );
    }

    #[test]
    fn typescript_source_and_config_are_in_watch_scope() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("scripts")).unwrap();
        std::fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        let watcher = FileWatcher::new(dir.path().to_path_buf()).unwrap();

        assert!(watcher.watch_dirs().contains(&dir.path().join("scripts")));
        assert!(
            watcher
                .config_paths()
                .contains(&dir.path().join("tsconfig.json"))
        );
        assert!(!watcher.watch_dirs().contains(&dir.path().to_path_buf()));
    }

    #[test]
    fn test_categorize_config_file() {
        let path = PathBuf::from("site.toml");
        let change_type = ChangeType::from_path(&path);
        assert_eq!(change_type, ChangeType::Config);
        assert_eq!(
            ChangeType::from_path(Path::new("tsconfig.json")),
            ChangeType::Config
        );
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

    /// #48 follow-up: a relative site_dir ("." is the CLI default) must
    /// still classify events. notify reports ABSOLUTE paths, so the
    /// site-relative strip in `from_notify_event` cannot use a relative
    /// `site_dir` as a prefix — the strip silently fails and the
    /// absolute path's first component is never a source directory, so
    /// every event classified `Unknown` and was dropped: hot reload
    /// appeared dead.
    #[test]
    fn from_notify_event_classifies_with_relative_site_dir() {
        // `taxus serve` in a site root passes site_dir = "."; with a
        // named relative dir (`-d mysite`) notify still reports absolute
        // paths. Both must classify.
        for site_dir in [PathBuf::from("."), PathBuf::from("mysite")] {
            let event = Event {
                kind: EventKind::Modify(notify::event::ModifyKind::Any),
                paths: vec![PathBuf::from("/abs/mysite/content/blog/post.md")],
                attrs: Default::default(),
            };
            let we = WatchEvent::from_notify_event(&site_dir, &event);
            assert_eq!(
                we.change_type,
                ChangeType::Content,
                "site_dir {site_dir:?}: absolute event path must classify"
            );
            assert!(we.should_rebuild());
        }
    }

    /// A bare `.` site_dir with an absolute event path: the strip cannot
    /// prefix-match, so classification falls back to locating the first
    /// source-directory component.
    ///
    /// The fixture is assembled from `std::path::MAIN_SEPARATOR` so it
    /// is a real multi-component path on every platform — a hard-coded
    /// Windows string has no separators on POSIX, where it parses as one
    /// component and the fallback (correctly) cannot fire. That failure
    /// mode reads exactly like a code bug, which is why the fixture
    /// cannot be a literal.
    #[test]
    fn from_notify_event_classifies_bare_dot_site_dir() {
        let sep = std::path::MAIN_SEPARATOR_STR;
        let abs = ["/home", "me", "site", "templates", "base.html"].join(sep);
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Any),
            paths: vec![PathBuf::from(abs)],
            attrs: Default::default(),
        };
        let we = WatchEvent::from_notify_event(Path::new("."), &event);
        assert_eq!(we.change_type, ChangeType::Template, "got {we:?}");
    }

    #[test]
    fn from_notify_event_classifies_configured_source_roots() {
        let site = tempfile::tempdir().unwrap();
        let root = site.path().canonicalize().unwrap();
        let roots = [
            (root.join("src/posts"), ChangeType::Content),
            (root.join("tpl"), ChangeType::Template),
            (root.join("assets/styles"), ChangeType::Style),
            (root.join("public"), ChangeType::Static),
            (root.join("scripts"), ChangeType::Script),
        ];

        for (relative, expected) in [
            ("src/posts/post.md", ChangeType::Content),
            ("tpl/base.html", ChangeType::Template),
            ("assets/styles/main.scss", ChangeType::Style),
            ("public/logo.svg", ChangeType::Static),
            ("scripts/main.ts", ChangeType::Script),
        ] {
            let absolute = root.join(relative);
            let event = Event {
                kind: EventKind::Modify(notify::event::ModifyKind::Any),
                paths: vec![absolute],
                attrs: Default::default(),
            };
            let watch_event = WatchEvent::from_notify_event_with_source_dirs(&root, &event, &roots);
            assert_eq!(watch_event.change_type, expected, "path: {relative}");
            assert_eq!(watch_event.paths, vec![PathBuf::from(relative)]);
        }

        let default_path = root.join("content/ignored.md");
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Any),
            paths: vec![default_path],
            attrs: Default::default(),
        };
        let watch_event = WatchEvent::from_notify_event_with_source_dirs(&root, &event, &roots);
        assert_eq!(watch_event.change_type, ChangeType::Unknown);
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
