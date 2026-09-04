// taxus-generator/src/serve/server.rs

//! Development server with live reload.
//!
//! This module provides the main development server that serves static files
//! and handles WebSocket connections for live reload.
//!
//! Static files are served by [`tower_http::services::ServeDir`], which provides:
//! - Automatic MIME type detection (no hand-maintained extension map)
//! - Proper `Content-Length` and `Last-Modified` headers
//! - Path traversal protection
//! - Directory index (`index.html`) resolution
//! - Precompressed file support (`.br`, `.gz`)
//!
//! A middleware layer is applied on top that:
//! - Rewrites extensionless paths (e.g. `/about`) to `/about.html` when the
//!   `.html` file exists on disk
//! - Injects the live-reload WebSocket `<script>` into HTML responses

use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A function that rebuilds the site. Returns Ok(()) on success or
/// an error message on failure
pub type RebuildFn = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use tokio::sync::broadcast;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};

use super::coordinator::RebuildCoordinator;
use super::error::ServeError;
use super::injector::inject_live_reload_script;
use super::watcher::FileWatcher;
use super::websocket::WebSocketMessage;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the development server.
#[derive(Debug, Clone)]
pub struct DevServerConfig {
    /// The port to listen on.
    pub port: u16,
    /// The output directory to serve.
    pub output_dir: PathBuf,
    /// The site directory to watch for changes.
    pub site_dir: PathBuf,
    /// Include draft pages in build.
    pub include_drafts: bool,
    /// Open the site in the browser after the initial build completes.
    pub open: bool,
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            output_dir: PathBuf::from("dist"),
            site_dir: PathBuf::from("."),
            include_drafts: false,
            open: false,
        }
    }
}

impl DevServerConfig {
    /// Create a new configuration with the specified port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Create a new configuration with the specified output directory.
    pub fn with_output_dir(mut self, output_dir: PathBuf) -> Self {
        self.output_dir = output_dir;
        self
    }

    /// Create a new configuration with the specified site directory.
    pub fn with_site_dir(mut self, site_dir: PathBuf) -> Self {
        self.site_dir = site_dir;
        self
    }

    /// Create a new configuration with drafts included.
    pub fn with_include_drafts(mut self, include: bool) -> Self {
        self.include_drafts = include;
        self
    }

    /// Create a new configuration that opens the browser after the initial build.
    pub fn with_open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// Shared state for the development server.
#[derive(Debug)]
pub struct ServerState {
    /// Broadcast channel for reload events.
    pub reload_tx: broadcast::Sender<WebSocketMessage>,
    /// The output directory being served.
    pub output_dir: PathBuf,
    /// Set to `true` once the initial build has completed.
    /// While `false`, the middleware serves a "Building…" page (503)
    /// instead of a bare 404.
    pub build_ready: Arc<AtomicBool>,
}

// ---------------------------------------------------------------------------
// Dev server
// ---------------------------------------------------------------------------

/// Development server with live reload support.
pub struct DevServer {
    config: DevServerConfig,
    rebuild: RebuildFn,
}

impl DevServer {
    /// Create a new development server.
    pub fn new(config: DevServerConfig, rebuild: RebuildFn) -> Self {
        Self { config, rebuild }
    }

    /// Build the Axum router.
    fn build_router(&self, state: Arc<ServerState>) -> Router {
        // ServeDir handles all static file serving: MIME types,
        // Last-Modified, Content-Length, index.html, and 404.html fallback.
        let serve_dir = ServeDir::new(&self.config.output_dir)
            .not_found_service(ServeFile::new(self.config.output_dir.join("404.html")));

        Router::new()
            // WebSocket endpoint for live reload
            .route("/__ws__", get(websocket_handler))
            // Favicon endpoint — browsers request /favicon.ico directly
            .route("/favicon.ico", get(favicon_handler))
            // Static files served by ServeDir, wrapped with our middleware
            // for clean-URL rewriting and live-reload injection.
            .fallback_service(serve_dir)
            .layer(from_fn_with_state(
                state.clone(),
                rewrite_and_inject_middleware,
            ))
            .with_state(state)
    }

    /// Run the development server with graceful shutdown.
    ///
    /// The TCP listener is bound **before** the initial build so that the
    /// server is reachable immediately.  Requests that arrive while the
    /// initial build is still running receive a "Building…" page (HTTP 503)
    /// that includes the live-reload WebSocket script and auto-reloads once
    /// the build completes.
    pub async fn run(self) -> Result<(), ServeError> {
        let addr: SocketAddr = ([0, 0, 0, 0], self.config.port).into();

        // Create broadcast channel for reload events
        let (reload_tx, _) = broadcast::channel(16);

        // Start file watcher
        let mut watcher = FileWatcher::new(self.config.site_dir.clone())?;
        watcher.start()?;
        let (watcher_guard, watch_events) = watcher.into_receiver();

        // Shared state — build_ready starts false so the middleware can
        // serve a "Building…" page until the initial build finishes.
        let build_ready = Arc::new(AtomicBool::new(false));
        let state = Arc::new(ServerState {
            reload_tx: reload_tx.clone(),
            output_dir: self.config.output_dir.clone(),
            build_ready: build_ready.clone(),
        });

        let app = self.build_router(state);

        // ── Bind FIRST — the server is reachable from this point ────────
        let listener =
            tokio::net::TcpListener::bind(addr)
                .await
                .map_err(|_| ServeError::PortInUse {
                    port: self.config.port,
                })?;

        info!("Development server listening on http://{}", addr);
        info!("Press Ctrl+C to stop");

        // Every rebuild — the initial one and every watcher-triggered one —
        // goes through this coordinator, which serialises them behind one
        // mutex and coalesces change events that arrive mid-build (#36).
        let coordinator = Arc::new(RebuildCoordinator::from_blocking(
            self.rebuild.clone(),
            reload_tx.clone(),
        ));

        // ── Spawn watcher task ──────────────────────────────────────────
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let coordinator_for_watcher = coordinator.clone();
        let watcher_handle = tokio::spawn(async move {
            coordinator_for_watcher.run(watch_events, shutdown_rx).await;
            // Keep the OS watcher registered for as long as the loop runs.
            drop(watcher_guard);
        });

        // ── Spawn initial build (runs concurrently with the server) ─────
        let build_ready_for_build = build_ready.clone();
        let open_browser = self.config.open;
        let port = self.config.port;

        tokio::spawn(async move {
            info!("Performing initial build...");
            // The coordinator broadcasts a failure to connected browsers;
            // only the logging is done here.
            match coordinator.build().await {
                Ok(_) => info!("Initial build complete"),
                Err(e) => warn!("Initial build failed: {}", e),
            }

            // Mark build as ready regardless of outcome so the "Building…"
            // page is no longer served (real 404.html takes over on error).
            build_ready_for_build.store(true, Ordering::Release);

            // Open browser now that the build is done and files are on disk.
            if open_browser {
                let url = format!("http://localhost:{}", port);
                if let Err(e) = webbrowser::open(&url) {
                    warn!("Failed to open browser: {}", e);
                }
            }
        });

        // ── Serve (blocking) ────────────────────────────────────────────
        let shutdown_signal = shutdown_signal();

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| ServeError::Server(e.to_string()))?;

        info!("Shutting down development server...");
        let _ = shutdown_tx.send(());
        let _ = watcher_handle.await;

        info!("Development server stopped");

        Ok(())
    }

    /// Get the server's port.
    pub fn port(&self) -> u16 {
        self.config.port
    }
}

// ---------------------------------------------------------------------------
// Middleware: clean-URL rewrite + live-reload injection
// ---------------------------------------------------------------------------

/// Middleware that:
///
/// 1. **Rewrites extensionless paths**: When a request comes in for `/about`
///    (no file extension) and `output_dir/about.html` exists on disk, the URI
///    is rewritten to `/about.html` before reaching [`ServeDir`].  Directory
///    index resolution (`/blog/` → `blog/index.html`) is left to [`ServeDir`].
///
/// 2. **Injects the live-reload script**: Any response whose `Content-Type`
///    contains `text/html` is buffered, the WebSocket script is injected
///    before `</body>`, and the modified body is re-emitted.
async fn rewrite_and_inject_middleware(
    State(state): State<Arc<ServerState>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path();

    // ── Clean-URL rewrite ────────────────────────────────────────────────
    // ServeDir already handles directory → index.html.  We only need to
    // handle the "extensionless → .html" case that static site generators
    // produce (e.g. `/about` → `about.html`).
    if is_extensionless_path(path) {
        let relative = path.trim_start_matches('/');
        let html_path = state.output_dir.join(format!("{}.html", relative));

        // Non-blocking check: if the .html file exists, rewrite the URI.
        if tokio::fs::metadata(&html_path).await.is_ok()
            && let Ok(new_uri) = format!("/{}.html", relative).parse()
        {
            *req.uri_mut() = new_uri;
        }
    }

    let response = next.run(req).await;

    // ── Building-page intercept ──────────────────────────────────────────
    // If the initial build hasn't completed yet, ServeDir has no files to
    // serve and returns 404.  Replace that with a styled "Building…" page
    // (503) that includes the live-reload script so the browser reloads
    // automatically once the build finishes.
    if response.status() == StatusCode::NOT_FOUND && !state.build_ready.load(Ordering::Relaxed) {
        return building_page();
    }

    // ── Live-reload injection ────────────────────────────────────────────
    inject_if_html(response).await
}

/// Returns `true` if `path` has no file extension and is not the root.
fn is_extensionless_path(path: &str) -> bool {
    !path.is_empty() && !path.ends_with('/') && Path::new(path).extension().is_none()
}

/// If the response is HTML, buffer the body, inject the live-reload script,
/// and return a new response preserving the original status code.
/// Otherwise return the response unchanged.
async fn inject_if_html(response: Response) -> Response {
    let is_html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|ct| ct.to_str().is_ok_and(|v| v.contains("text/html")));

    if !is_html {
        return response;
    }

    let status = response.status();

    // Buffer the body — acceptable for a dev server serving local HTML files.
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body collection failed")
        .to_bytes();

    let html = String::from_utf8_lossy(&bytes);
    let injected = inject_live_reload_script(&html);

    // Rebuild the response preserving the original status code.
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CONTENT_LENGTH, injected.len())
        .body(Body::from(injected))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Building page
// ---------------------------------------------------------------------------

/// Return a styled "Building…" page (503) that includes the live-reload
/// WebSocket script.  Served while the initial build is in progress.
fn building_page() -> Response {
    let html = r#"<!DOCTYPE html>
<html>
<head>
<title>Building… – taxus</title>
<style>
  body {
    margin: 0; display: flex; align-items: center; justify-content: center;
    min-height: 100vh; font-family: system-ui, sans-serif;
    background: #1a1a2e; color: #e0e0e0;
  }
  .wrap { text-align: center; }
  h1 { font-size: 1.5rem; margin-bottom: .5rem; }
  p { color: #888; }
</style>
</head>
<body>
<div class="wrap">
  <h1>Building…</h1>
  <p>This page will reload automatically when the build completes.</p>
</div>
</body>
</html>"#
        .to_string();
    let injected = inject_live_reload_script(&html);
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CONTENT_LENGTH, injected.len())
        .header("retry-after", "2")
        .body(Body::from(injected))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Favicon handler
// ---------------------------------------------------------------------------

/// Handle favicon.ico requests.
///
/// Browsers often request /favicon.ico directly. This handler looks for
/// favicon files in the static directory and serves them.
async fn favicon_handler(State(state): State<Arc<ServerState>>) -> Response {
    let favicon_paths = [
        state.output_dir.join("static").join("favicon.ico"),
        state.output_dir.join("static").join("favicon.png"),
    ];

    for favicon_path in &favicon_paths {
        if let Ok(content) = tokio::fs::read(favicon_path).await {
            let content_type = if favicon_path.extension().is_some_and(|ext| ext == "png") {
                "image/png"
            } else {
                "image/x-icon"
            };

            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=86400")
                .body(Body::from(content))
                .unwrap();
        }
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap()
}

// ---------------------------------------------------------------------------
// WebSocket handler
// ---------------------------------------------------------------------------

/// Handle WebSocket upgrade requests.
async fn websocket_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle a WebSocket connection.
async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: Arc<ServerState>) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Send connected message
    let connected_msg = WebSocketMessage::Connected {
        server: "taxus".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = ws_tx
            .send(axum::extract::ws::Message::Text(json.into()))
            .await;
    }

    // Subscribe to reload events
    let mut reload_rx = state.reload_tx.subscribe();

    let recv_task = async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(axum::extract::ws::Message::Ping(_)) => {}
                Ok(axum::extract::ws::Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    };

    let send_task = async move {
        while let Ok(msg) = reload_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg)
                && ws_tx
                    .send(axum::extract::ws::Message::Text(json.into()))
                    .await
                    .is_err()
            {
                break;
            }
        }
    };

    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }
}

// ---------------------------------------------------------------------------
// Shutdown signal
// ---------------------------------------------------------------------------

/// Create a shutdown signal that triggers on Ctrl+C.
fn shutdown_signal() -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            use tokio::signal::unix::{SignalKind, signal};
            signal(SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("Shutdown signal received");
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, Bytes};
    use axum::http::{HeaderMap, StatusCode, header};
    use http_body_util::BodyExt;
    use tempfile::TempDir;
    use tower::ServiceExt;

    /// Create a temporary output directory with common test fixtures:
    ///
    /// ```text
    /// output_dir/
    ///   index.html              — simple HTML page
    ///   about.html              — simple HTML page
    ///   blog/
    ///     index.html            — blog listing
    ///   styles/
    ///     main.css              — CSS file
    ///   scripts/
    ///     app.js                — JS file
    ///   images/
    ///     logo.png              — binary "image"
    ///   404.html                — custom 404 page
    ///   static/
    ///     favicon.ico           — favicon binary
    /// ```
    fn create_test_output_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Helper to write a file
        let write_file = |rel_path: &str, content: &[u8]| {
            let full = dir.path().join(rel_path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, content).unwrap();
        };

        write_file(
            "index.html",
            b"<!DOCTYPE html><html><head><title>Home</title></head><body><h1>Home</h1></body></html>",
        );
        write_file(
            "about.html",
            b"<!DOCTYPE html><html><head><title>About</title></head><body><p>About us</p></body></html>",
        );
        write_file(
            "blog/index.html",
            b"<!DOCTYPE html><html><body><h1>Blog</h1></body></html>",
        );
        write_file("styles/main.css", b"h1 { color: red; }");
        write_file("scripts/app.js", b"console.log('hello');");
        write_file("images/logo.png", b"\x89PNG\r\n\x1a\nfake-png-data");
        write_file(
            "404.html",
            b"<!DOCTYPE html><html><body><h1>Not Found</h1></body></html>",
        );
        write_file("static/favicon.ico", b"fake-ico-data");

        dir
    }

    /// Build a test router (no WebSocket, no file watcher) that serves from
    /// the given output directory.  The build is assumed to be complete
    /// (`build_ready = true`).
    fn test_router(output_dir: &Path) -> Router {
        test_router_with_build_state(output_dir, true)
    }

    /// Build a test router with an explicit build-ready state.
    fn test_router_with_build_state(output_dir: &Path, build_ready: bool) -> Router {
        let (reload_tx, _) = broadcast::channel(16);
        let state = Arc::new(ServerState {
            reload_tx,
            output_dir: output_dir.to_path_buf(),
            build_ready: Arc::new(AtomicBool::new(build_ready)),
        });

        let serve_dir = ServeDir::new(output_dir)
            .not_found_service(ServeFile::new(output_dir.join("404.html")));

        Router::new()
            .route("/favicon.ico", get(favicon_handler))
            .fallback_service(serve_dir)
            .layer(from_fn_with_state(
                state.clone(),
                rewrite_and_inject_middleware,
            ))
            .with_state(state)
    }

    /// Send a GET request to the router and return (status, headers, body bytes).
    async fn send_get(router: &Router, path: &str) -> (StatusCode, HeaderMap, Bytes) {
        let req = Request::builder().uri(path).body(Body::empty()).unwrap();

        let response = router.clone().oneshot(req).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, headers, body)
    }

    fn body_string(bytes: &Bytes) -> String {
        String::from_utf8_lossy(bytes).to_string()
    }

    // =========================================================================
    // Configuration tests
    // =========================================================================

    #[test]
    fn test_default_config() {
        let config = DevServerConfig::default();
        assert_eq!(config.port, 3000);
        assert_eq!(config.output_dir, PathBuf::from("dist"));
    }

    #[test]
    fn test_config_with_port() {
        let config = DevServerConfig::default().with_port(8080);
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_config_with_output_dir() {
        let config = DevServerConfig::default().with_output_dir(PathBuf::from("output"));
        assert_eq!(config.output_dir, PathBuf::from("output"));
    }

    #[test]
    fn test_config_with_site_dir() {
        let config = DevServerConfig::default().with_site_dir(PathBuf::from("mysite"));
        assert_eq!(config.site_dir, PathBuf::from("mysite"));
    }

    #[test]
    fn test_server_creation() {
        let config = DevServerConfig::default();
        let rebuild: RebuildFn = Arc::new(|| Ok(()));
        let server = DevServer::new(config, rebuild);
        assert_eq!(server.port(), 3000);
    }

    #[test]
    fn test_server_state_creation() {
        let (tx, mut rx) = broadcast::channel(16);
        let state = ServerState {
            reload_tx: tx,
            output_dir: PathBuf::from("dist"),
            build_ready: Arc::new(AtomicBool::new(true)),
        };

        let msg = WebSocketMessage::Connected {
            server: "test".to_string(),
        };
        assert!(state.reload_tx.send(msg).is_ok());
        let received = rx.try_recv();
        assert!(received.is_ok());
    }

    // =========================================================================
    // Helper function tests
    // =========================================================================

    #[test]
    fn test_is_extensionless_path() {
        assert!(is_extensionless_path("/about"));
        assert!(is_extensionless_path("/blog/2024/post"));
        assert!(!is_extensionless_path("/about.html"));
        assert!(!is_extensionless_path("/styles/main.css"));
        assert!(!is_extensionless_path("/"));
        assert!(!is_extensionless_path(""));
        assert!(!is_extensionless_path("/blog/"));
    }

    // =========================================================================
    // Integration tests: clean-URL resolution
    // =========================================================================

    #[tokio::test]
    async fn test_root_serves_index_html() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (status, headers, body) = send_get(&router, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            headers
                .get(header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap()
                .contains("text/html")
        );
        let html = body_string(&body);
        assert!(html.contains("<h1>Home</h1>"));
    }

    #[tokio::test]
    async fn test_clean_url_resolution() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        // /about has no extension; about.html exists → should serve it
        let (status, _headers, body) = send_get(&router, "/about").await;
        assert_eq!(status, StatusCode::OK);
        let html = body_string(&body);
        assert!(html.contains("<title>About</title>"));
        assert!(html.contains("<p>About us</p>"));
    }

    #[tokio::test]
    async fn test_clean_url_nested() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        // /blog has no extension but blog/index.html exists (directory index)
        // ServeDir handles this automatically.
        let (status, _headers, body) = send_get(&router, "/blog/").await;
        assert_eq!(status, StatusCode::OK);
        let html = body_string(&body);
        assert!(html.contains("<h1>Blog</h1>"));
    }

    #[tokio::test]
    async fn test_direct_html_request() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (status, _headers, body) = send_get(&router, "/about.html").await;
        assert_eq!(status, StatusCode::OK);
        let html = body_string(&body);
        assert!(html.contains("<title>About</title>"));
    }

    // =========================================================================
    // Integration tests: live-reload injection
    // =========================================================================

    #[tokio::test]
    async fn test_html_response_has_live_reload_script() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (_status, _headers, body) = send_get(&router, "/about.html").await;
        let html = body_string(&body);
        assert!(html.contains("__ws__"));
        assert!(html.contains("WebSocket"));
        assert!(html.contains("location.reload"));
    }

    #[tokio::test]
    async fn test_injected_script_appears_before_closing_body() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (_status, _headers, body) = send_get(&router, "/about.html").await;
        let html = body_string(&body);
        let script_pos = html.find("__ws__").expect("script marker not found");
        let body_close = html.find("</body>").expect("</body> not found");
        assert!(
            script_pos < body_close,
            "live-reload script must appear before </body>"
        );
    }

    #[tokio::test]
    async fn test_clean_url_injected_with_live_reload() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (_status, _headers, body) = send_get(&router, "/about").await;
        let html = body_string(&body);
        assert!(html.contains("__ws__"));
    }

    #[tokio::test]
    async fn test_root_page_injected_with_live_reload() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (_status, _headers, body) = send_get(&router, "/").await;
        let html = body_string(&body);
        assert!(html.contains("__ws__"));
    }

    // =========================================================================
    // Integration tests: non-HTML files pass through
    // =========================================================================

    #[tokio::test]
    async fn test_css_served_with_correct_mime() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (status, headers, body) = send_get(&router, "/styles/main.css").await;
        assert_eq!(status, StatusCode::OK);
        let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
        assert!(ct.contains("text/css"), "expected text/css, got {ct}");
        assert_eq!(body_string(&body), "h1 { color: red; }");
        // No live-reload injection
        let raw = body_string(&body);
        assert!(!raw.contains("__ws__"));
    }

    #[tokio::test]
    async fn test_js_served_with_correct_mime() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (status, headers, body) = send_get(&router, "/scripts/app.js").await;
        assert_eq!(status, StatusCode::OK);
        let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
        assert!(
            ct.contains("javascript") || ct.contains("text/javascript"),
            "expected javascript, got {ct}"
        );
        assert_eq!(body_string(&body), "console.log('hello');");
        assert!(!body_string(&body).contains("__ws__"));
    }

    #[tokio::test]
    async fn test_png_served_with_correct_mime() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (status, headers, _body) = send_get(&router, "/images/logo.png").await;
        assert_eq!(status, StatusCode::OK);
        let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
        assert!(ct.contains("image/png"), "expected image/png, got {ct}");
    }

    #[tokio::test]
    async fn test_svg_served_with_correct_mime() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        // Create an SVG on the fly
        let svg_path = dir.path().join("images/icon.svg");
        std::fs::write(&svg_path, "<svg xmlns='http://www.w3.org/2000/svg'></svg>").unwrap();

        let (status, headers, _body) = send_get(&router, "/images/icon.svg").await;
        assert_eq!(status, StatusCode::OK);
        let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
        assert!(ct.contains("svg"), "expected image/svg+xml, got {ct}");
    }

    #[tokio::test]
    async fn test_wasm_served_with_correct_mime() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let wasm_path = dir.path().join("pkg/app.wasm");
        std::fs::create_dir_all(wasm_path.parent().unwrap()).unwrap();
        std::fs::write(&wasm_path, b"\x00asmfake").unwrap();

        let (status, headers, _body) = send_get(&router, "/pkg/app.wasm").await;
        assert_eq!(status, StatusCode::OK);
        let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
        assert!(
            ct.contains("wasm") || ct.contains("octet-stream"),
            "expected wasm, got {ct}"
        );
    }

    #[tokio::test]
    async fn test_json_served_with_correct_mime() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let json_path = dir.path().join("data/manifest.json");
        std::fs::create_dir_all(json_path.parent().unwrap()).unwrap();
        std::fs::write(&json_path, r#"{"name":"taxus"}"#).unwrap();

        let (status, headers, body) = send_get(&router, "/data/manifest.json").await;
        assert_eq!(status, StatusCode::OK);
        let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
        assert!(ct.contains("json"), "expected json, got {ct}");
        assert_eq!(body_string(&body), r#"{"name":"taxus"}"#);
        assert!(!body_string(&body).contains("__ws__"));
    }

    // =========================================================================
    // Integration tests: 404 handling
    // =========================================================================

    #[tokio::test]
    async fn test_404_serves_custom_404_html() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (status, _headers, body) = send_get(&router, "/nonexistent-page").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let html = body_string(&body);
        assert!(html.contains("<h1>Not Found</h1>"));
        // 404.html should also get live-reload injection
        assert!(html.contains("__ws__"));
    }

    #[tokio::test]
    async fn test_404_html_has_injected_script() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (_status, _headers, body) = send_get(&router, "/no-such-file.js").await;
        let html = body_string(&body);
        assert!(html.contains("<h1>Not Found</h1>"));
        assert!(html.contains("__ws__"));
    }

    #[tokio::test]
    async fn test_404_without_custom_file_serves_empty_404() {
        let dir = TempDir::new().unwrap();
        // No 404.html, no index.html — just a CSS file
        std::fs::create_dir_all(dir.path().join("styles")).unwrap();
        std::fs::write(dir.path().join("styles/main.css"), b"body {}").unwrap();

        let (reload_tx, _) = broadcast::channel(16);
        let state = Arc::new(ServerState {
            reload_tx,
            output_dir: dir.path().to_path_buf(),
            build_ready: Arc::new(AtomicBool::new(true)),
        });
        // ServeDir without a 404.html fallback — uses DefaultServeDirFallback.
        // For a missing file, ServeDir returns 404 with an empty body.
        let serve_dir = ServeDir::new(dir.path());
        let router: Router = Router::new()
            .fallback_service(serve_dir)
            .layer(from_fn_with_state(
                state.clone(),
                rewrite_and_inject_middleware,
            ))
            .with_state(state);

        let (status, _headers, body) = send_get(&router, "/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // Default 404 has an empty body — no injection happens
        assert!(
            body.is_empty(),
            "default ServeDir 404 should have empty body"
        );
    }

    // =========================================================================
    // Integration tests: ServeDir-provided headers
    // =========================================================================

    #[tokio::test]
    async fn test_html_response_has_content_length() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (_status, headers, body) = send_get(&router, "/about.html").await;
        // inject_if_html rebuilds the response with the correct Content-Length
        assert!(
            headers.get(header::CONTENT_LENGTH).is_some(),
            "HTML response should have Content-Length"
        );
        // Content-Length should match actual body
        let cl: usize = headers
            .get(header::CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(cl, body.len());
    }

    #[tokio::test]
    async fn test_static_file_has_content_length() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (_status, headers, body) = send_get(&router, "/styles/main.css").await;
        assert!(
            headers.get(header::CONTENT_LENGTH).is_some(),
            "CSS response should have Content-Length"
        );
        let cl: usize = headers
            .get(header::CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(cl, body.len());
    }

    #[tokio::test]
    async fn test_last_modified_present() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (_status, headers, _body) = send_get(&router, "/styles/main.css").await;
        // ServeDir sets Last-Modified based on file metadata
        assert!(
            headers.get(header::LAST_MODIFIED).is_some(),
            "ServeDir should set Last-Modified header"
        );
    }

    // =========================================================================
    // Integration tests: favicon handler
    // =========================================================================

    #[tokio::test]
    async fn test_favicon_served_from_static() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        let (status, headers, body) = send_get(&router, "/favicon.ico").await;
        assert_eq!(status, StatusCode::OK);
        let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
        assert_eq!(ct, "image/x-icon");
        assert_eq!(body.as_ref(), b"fake-ico-data");
        // Has cache-control
        assert_eq!(
            headers
                .get(header::CACHE_CONTROL)
                .unwrap()
                .to_str()
                .unwrap(),
            "public, max-age=86400"
        );
    }

    #[tokio::test]
    async fn test_favicon_404_when_missing() {
        let dir = TempDir::new().unwrap();
        // No static directory at all
        let (reload_tx, _) = broadcast::channel(16);
        let state = Arc::new(ServerState {
            reload_tx,
            output_dir: dir.path().to_path_buf(),
            build_ready: Arc::new(AtomicBool::new(true)),
        });
        let router: Router = Router::new()
            .fallback_service(ServeDir::new(dir.path()))
            .layer(from_fn_with_state(
                state.clone(),
                rewrite_and_inject_middleware,
            ))
            .with_state(state);

        // The favicon route is not registered, so it falls through to ServeDir
        // which will 404.
        let (status, _headers, _body) = send_get(&router, "/favicon.ico").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // =========================================================================
    // Integration tests: extensionless non-HTML fallback
    // =========================================================================

    #[tokio::test]
    async fn test_nonexistent_extensionless_path_404s() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        // /nope has no extension, nope.html doesn't exist → 404
        let (status, _headers, body) = send_get(&router, "/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // Should serve custom 404.html
        assert!(body_string(&body).contains("Not Found"));
    }

    #[tokio::test]
    async fn test_extensionless_path_does_not_rewrite_if_no_html() {
        let dir = create_test_output_dir();
        let router = test_router(dir.path());

        // /styles/main has no extension; styles/main.html doesn't exist.
        // The middleware should NOT rewrite; ServeDir handles the 404.
        let (status, _headers, _body) = send_get(&router, "/styles/main").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // =========================================================================
    // Integration tests: building-page behavior
    // =========================================================================

    #[tokio::test]
    async fn test_building_page_served_when_build_not_ready() {
        let dir = TempDir::new().unwrap();
        // Empty directory — no files at all.
        let router = test_router_with_build_state(dir.path(), false);

        let (status, headers, body) = send_get(&router, "/").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let html = body_string(&body);
        assert!(
            html.contains("Building"),
            "building page should contain 'Building'"
        );
        assert!(
            html.contains("__ws__"),
            "building page should include live-reload script"
        );
        assert_eq!(headers.get("retry-after").unwrap().to_str().unwrap(), "2");
    }

    #[tokio::test]
    async fn test_building_page_intercepts_nonexistent_path() {
        let dir = TempDir::new().unwrap();
        let router = test_router_with_build_state(dir.path(), false);

        let (status, _headers, body) = send_get(&router, "/any/path").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body_string(&body).contains("Building"));
    }

    #[tokio::test]
    async fn test_normal_404_after_build_ready() {
        let dir = create_test_output_dir();
        // build_ready = true → normal ServeDir behavior, no building-page intercept
        let router = test_router_with_build_state(dir.path(), true);

        let (status, _headers, body) = send_get(&router, "/nonexistent").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body_string(&body).contains("Not Found"));
    }
}
