//! Development server with live reload.
//!
//! This module provides the main development server that serves static files
//! and handles WebSocket connections for live reload.

use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use super::error::ServeError;
use super::injector::inject_live_reload_script;
use super::watcher::FileWatcher;
use super::websocket::{ReloadEvent, WebSocketMessage};
use crate::build::SiteBuilder;

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
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            output_dir: PathBuf::from("dist"),
            site_dir: PathBuf::from("."),
            include_drafts: false,
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
}

/// Shared state for the development server.
#[derive(Debug)]
pub struct ServerState {
    /// Broadcast channel for reload events.
    pub reload_tx: broadcast::Sender<WebSocketMessage>,
    /// The output directory being served.
    pub output_dir: PathBuf,
}

/// Development server with live reload support.
pub struct DevServer {
    config: DevServerConfig,
}

impl DevServer {
    /// Create a new development server.
    pub fn new(config: DevServerConfig) -> Self {
        Self { config }
    }

    /// Build the Axum router.
    fn build_router(&self, state: Arc<ServerState>) -> Router {
        // Static file service with HTML injection for live reload
        let static_service =
            HtmlInjectService::new(state.output_dir.clone(), state.reload_tx.clone());

        Router::new()
            // WebSocket endpoint for live reload
            .route("/__ws__", get(websocket_handler))
            // Favicon endpoint - serve from static/favicon.png or static/favicon.ico
            .route("/favicon.ico", get(favicon_handler))
            // Static files with HTML injection
            .fallback_service(static_service)
            .with_state(state)
    }

    /// Run the development server with graceful shutdown.
    pub async fn run(self) -> Result<(), ServeError> {
        let addr: SocketAddr = ([0, 0, 0, 0], self.config.port).into();

        // Create broadcast channel for reload events
        let (reload_tx, _) = broadcast::channel(16);

        // Perform initial build
        info!("Performing initial build...");
        match self.rebuild() {
            Ok(_) => info!("Initial build complete"),
            Err(e) => {
                warn!("Initial build failed: {}", e);
                // Send error to connected clients
                let _ = reload_tx.send(WebSocketMessage::Error {
                    message: format!("Build failed: {}", e),
                });
            }
        }

        // Start file watcher
        let mut watcher = FileWatcher::new(self.config.site_dir.clone())?;
        watcher.start()?;

        let state = Arc::new(ServerState {
            reload_tx: reload_tx.clone(),
            output_dir: self.config.output_dir.clone(),
        });

        let app = self.build_router(state);

        let listener =
            tokio::net::TcpListener::bind(addr)
                .await
                .map_err(|_| ServeError::PortInUse {
                    port: self.config.port,
                })?;

        info!("Development server listening on http://{}", addr);
        info!("Press Ctrl+C to stop");

        // Create shutdown signal
        let shutdown_signal = shutdown_signal();

        // Spawn the watcher task with shutdown awareness
        let site_dir = self.config.site_dir.clone();
        let include_drafts = self.config.include_drafts;
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let watcher_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = &mut shutdown_rx => {
                        info!("File watcher shutting down...");
                        break;
                    }
                    // Handle file changes
                    result = watcher.recv() => {
                        match result {
                            Some(event) => {
                                info!("Change detected: {:?}", event.change_type);

                                // Trigger rebuild
                                match Self::do_rebuild(&site_dir, include_drafts) {
                                    Ok(_) => {
                                        // Send reload notification
                                        let files: Vec<String> = event
                                            .paths
                                            .iter()
                                            .map(|p| p.to_string_lossy().to_string())
                                            .collect();
                                        let reload_event = ReloadEvent::new(event.change_type, files);
                                        let _ = reload_tx.send(WebSocketMessage::Reload(reload_event));
                                    }
                                    Err(e) => {
                                        error!("Build failed: {}", e);
                                        let _ = reload_tx.send(WebSocketMessage::Error {
                                            message: format!("Build failed: {}", e),
                                        });
                                    }
                                }
                            }
                            None => {
                                // Channel closed, exit the loop
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Run server with graceful shutdown
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| ServeError::Server(e.to_string()))?;

        // Signal the watcher to stop and wait for it
        info!("Shutting down development server...");
        let _ = shutdown_tx.send(());
        let _ = watcher_handle.await;

        info!("Development server stopped");

        Ok(())
    }

    /// Perform a rebuild of the site.
    fn rebuild(&self) -> Result<(), String> {
        Self::do_rebuild(&self.config.site_dir, self.config.include_drafts)
    }

    /// Internal rebuild implementation.
    fn do_rebuild(site_dir: &Path, include_drafts: bool) -> Result<(), String> {
        SiteBuilder::from_dir(site_dir)
            .map_err(|e| e.to_string())?
            .include_drafts(include_drafts)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get the server's port.
    pub fn port(&self) -> u16 {
        self.config.port
    }

    /// Get a clone of the reload sender.
    pub fn reload_sender(&self) -> Option<broadcast::Sender<WebSocketMessage>> {
        // This would be populated after run() is called
        None
    }
}

/// Custom service that serves static files and injects live reload script into HTML.
///
/// This service wraps `ServeDir` and intercepts HTML responses to inject the
/// live reload WebSocket script before the closing `</body>` tag.
#[derive(Clone)]
struct HtmlInjectService {
    output_dir: PathBuf,
    reload_tx: broadcast::Sender<WebSocketMessage>,
}

impl HtmlInjectService {
    fn new(output_dir: PathBuf, reload_tx: broadcast::Sender<WebSocketMessage>) -> Self {
        Self {
            output_dir,
            reload_tx,
        }
    }
}

impl<B> tower::Service<axum::http::Request<B>> for HtmlInjectService
where
    B: Send + 'static,
{
    type Response = axum::response::Response;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::http::Request<B>) -> Self::Future {
        let output_dir = self.output_dir.clone();
        let _reload_tx = self.reload_tx.clone();

        Box::pin(async move {
            // Get the path from the URI
            let path = req.uri().path();

            // Normalize the path - remove leading slash
            let relative_path = path.trim_start_matches('/');

            // Handle root path
            let file_path = if relative_path.is_empty() || relative_path == "/" {
                output_dir.join("index.html")
            } else {
                output_dir.join(relative_path)
            };

            // Check if it's an HTML file request
            let is_html_request = file_path.extension().is_none_or(|ext| ext == "html");

            // Try to read the file
            if is_html_request {
                // Try the path as-is first, then try with .html extension
                let paths_to_try = if file_path.extension().is_some_and(|ext| ext == "html") {
                    vec![file_path.clone()]
                } else {
                    vec![
                        file_path.with_extension("html"),
                        file_path.join("index.html"),
                    ]
                };

                for try_path in paths_to_try {
                    if let Ok(content) = tokio::fs::read_to_string(&try_path).await {
                        // Check if it's HTML content
                        if content.contains("<!DOCTYPE html") || content.contains("<html") {
                            let injected = inject_live_reload_script(&content);
                            return Ok(axum::response::Html(injected).into_response());
                        }
                    }
                }
            }

            // Fall back to serving the file directly with proper content type
            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    // Determine content type based on extension
                    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
                        Some("html") => "text/html; charset=utf-8",
                        Some("css") => "text/css; charset=utf-8",
                        Some("js") => "application/javascript; charset=utf-8",
                        Some("json") => "application/json; charset=utf-8",
                        Some("png") => "image/png",
                        Some("jpg") | Some("jpeg") => "image/jpeg",
                        Some("gif") => "image/gif",
                        Some("svg") => "image/svg+xml",
                        Some("ico") => "image/x-icon",
                        Some("woff") | Some("woff2") => "font/woff2",
                        Some("ttf") => "font/ttf",
                        Some("eot") => "application/vnd.ms-fontobject",
                        _ => "application/octet-stream",
                    };

                    let response = axum::response::Response::builder()
                        .status(axum::http::StatusCode::OK)
                        .header(axum::http::header::CONTENT_TYPE, content_type)
                        .body(axum::body::Body::from(content))
                        .unwrap();

                    Ok(response)
                }
                Err(_) => {
                    // File not found - try to serve 404.html
                    let not_found_path = output_dir.join("404.html");

                    if let Ok(content) = tokio::fs::read_to_string(&not_found_path).await {
                        // Inject live reload script if it's HTML
                        if content.contains("<!DOCTYPE html") || content.contains("<html") {
                            let injected = inject_live_reload_script(&content);
                            let response = axum::response::Response::builder()
                                .status(axum::http::StatusCode::NOT_FOUND)
                                .header(
                                    axum::http::header::CONTENT_TYPE,
                                    "text/html; charset=utf-8",
                                )
                                .body(axum::body::Body::from(injected))
                                .unwrap();
                            return Ok(response);
                        }
                        // Return as-is with 404 status
                        let response = axum::response::Response::builder()
                            .status(axum::http::StatusCode::NOT_FOUND)
                            .header(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")
                            .body(axum::body::Body::from(content))
                            .unwrap();
                        return Ok(response);
                    }

                    // No 404.html - fall back to plain text
                    let response = axum::response::Response::builder()
                        .status(axum::http::StatusCode::NOT_FOUND)
                        .body(axum::body::Body::from("Not Found"))
                        .unwrap();

                    Ok(response)
                }
            }
        })
    }
}

/// Handle favicon.ico requests.
///
/// Browsers often request /favicon.ico directly. This handler looks for
/// favicon files in the static directory and serves them.
async fn favicon_handler(State(state): State<Arc<ServerState>>) -> Response {
    // Try common favicon locations in order
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

            return axum::response::Response::builder()
                .status(axum::http::StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, content_type)
                .header(axum::http::header::CACHE_CONTROL, "public, max-age=86400")
                .body(axum::body::Body::from(content))
                .unwrap();
        }
    }

    // No favicon found - return 404
    axum::response::Response::builder()
        .status(axum::http::StatusCode::NOT_FOUND)
        .body(axum::body::Body::from("Not Found"))
        .unwrap()
}

/// Handle WebSocket upgrade requests.
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle a WebSocket connection.
async fn handle_websocket(socket: WebSocket, state: Arc<ServerState>) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Send connected message
    let connected_msg = WebSocketMessage::Connected {
        server: "taxus".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = ws_tx.send(Message::Text(json.into())).await;
    }

    // Subscribe to reload events
    let mut reload_rx = state.reload_tx.subscribe();

    // Spawn a task to handle incoming messages (keep-alive)
    let recv_task = async move {
        while let Some(msg) = ws_rx.next().await {
            match msg {
                Ok(Message::Ping(_)) => {
                    // Respond to ping with pong (handled automatically by axum)
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    };

    // Spawn a task to send reload events
    let send_task = async move {
        while let Ok(msg) = reload_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg)
                && ws_tx.send(Message::Text(json.into())).await.is_err()
            {
                break;
            }
        }
    };

    // Run both tasks
    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }
}

/// Create a shutdown signal that triggers on Ctrl+C.
///
/// This function returns a future that resolves when the user presses Ctrl+C,
/// enabling graceful shutdown of the server.
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let server = DevServer::new(config);
        assert_eq!(server.port(), 3000);
    }

    #[test]
    fn test_server_state_creation() {
        let (tx, mut rx) = broadcast::channel(16);
        let state = ServerState {
            reload_tx: tx,
            output_dir: PathBuf::from("dist"),
        };

        // Verify we can send a message (need a receiver for broadcast to work)
        let msg = WebSocketMessage::Connected {
            server: "test".to_string(),
        };
        assert!(state.reload_tx.send(msg).is_ok());

        // Verify the receiver gets the message
        let received = rx.try_recv();
        assert!(received.is_ok());
    }
}
