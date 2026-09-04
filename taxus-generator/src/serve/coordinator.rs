// taxus-generator/src/serve/coordinator.rs

//! Serialises and coalesces site rebuilds for the dev server.
//!
//! [`DevServer::run`](super::DevServer::run) has two independent sources of
//! rebuilds: the initial build performed at startup and the file-watcher
//! loop, which wants one per change event.  Running those concurrently lets
//! two builds write to the output directory at the same time (partial files,
//! one build's `remove_dir_all` racing another's writes).  A burst of change
//! events also queues one full rebuild per event.
//!
//! [`RebuildCoordinator`] owns a single [`tokio::sync::Mutex`] that every
//! build goes through, so at most one build is ever in flight, and its watch
//! loop drains every event that queued up while a build was running into one
//! follow-up rebuild instead of N.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tracing::{error, info};

use super::server::RebuildFn;
use super::watcher::WatchEvent;
use super::websocket::{ReloadEvent, WebSocketMessage};

/// A boxed, `Send` future returned by an [`AsyncRebuildFn`].
pub(crate) type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// An asynchronous rebuild function.
///
/// The production server wraps its blocking [`RebuildFn`] in
/// `spawn_blocking` (see [`RebuildCoordinator::from_blocking`]); tests supply
/// a future they can hold open to observe what the coordinator does while a
/// build is in flight.
pub(crate) type AsyncRebuildFn = Arc<dyn Fn() -> BoxFuture<Result<(), String>> + Send + Sync>;

/// Serialises every rebuild behind one mutex and coalesces queued events.
pub(crate) struct RebuildCoordinator {
    rebuild: AsyncRebuildFn,
    reload_tx: broadcast::Sender<WebSocketMessage>,
    lock: Mutex<()>,
}

impl RebuildCoordinator {
    /// Create a coordinator around an asynchronous rebuild function.
    pub(crate) fn new(
        rebuild: AsyncRebuildFn,
        reload_tx: broadcast::Sender<WebSocketMessage>,
    ) -> Self {
        Self {
            rebuild,
            reload_tx,
            lock: Mutex::new(()),
        }
    }

    /// Create a coordinator around a blocking [`RebuildFn`].
    ///
    /// Each build runs on the blocking thread pool via
    /// [`tokio::task::spawn_blocking`], exactly as `DevServer::run` did
    /// before the coordinator existed.
    pub(crate) fn from_blocking(
        rebuild: RebuildFn,
        reload_tx: broadcast::Sender<WebSocketMessage>,
    ) -> Self {
        let rebuild: AsyncRebuildFn = Arc::new(move || {
            let rebuild = rebuild.clone();
            Box::pin(async move {
                tokio::task::spawn_blocking(move || (rebuild)())
                    .await
                    .unwrap_or_else(|e| Err(format!("Build task failed: {}", e)))
            })
        });
        Self::new(rebuild, reload_tx)
    }

    /// Run one build, serialised against every other build that goes
    /// through this coordinator.
    ///
    /// On failure a [`WebSocketMessage::Error`] is broadcast to connected
    /// browsers and the error is returned so the caller can log it in its
    /// own words.  Success is *not* broadcast: the watch loop sends a
    /// [`WebSocketMessage::Reload`] carrying the changed files, while the
    /// initial build has nothing to reload.
    pub(crate) async fn build(&self) -> Result<(), String> {
        let _guard = self.lock.lock().await;
        self.run_build().await
    }

    /// Run the rebuild function and broadcast on failure. The caller must
    /// hold `self.lock`.
    async fn run_build(&self) -> Result<(), String> {
        let result = (self.rebuild)().await;
        if let Err(e) = &result {
            let _ = self.reload_tx.send(WebSocketMessage::Error {
                message: format!("Build failed: {}", e),
            });
        }
        result
    }

    /// Consume watch events until `shutdown` fires or the channel closes,
    /// rebuilding once per burst of events.
    ///
    /// After an event is received the build lock is taken, and every event
    /// that queued up in the meantime (while an earlier build, including the
    /// initial one, held the lock) is drained and folded into this single
    /// rebuild.  Events that arrive *during* the build are picked up by the
    /// next iteration, again as one rebuild.
    pub(crate) async fn run(
        &self,
        mut events: mpsc::Receiver<WatchEvent>,
        mut shutdown: oneshot::Receiver<()>,
    ) {
        loop {
            let event = tokio::select! {
                _ = &mut shutdown => {
                    info!("File watcher shutting down...");
                    break;
                }
                event = events.recv() => match event {
                    Some(event) => event,
                    None => break,
                },
            };

            info!("Change detected: {:?}", event.change_type);

            // Take the build lock *before* draining so that everything that
            // queued up while another build (the initial one, or the previous
            // iteration's) held the lock is folded into this single rebuild.
            let guard = self.lock.lock().await;
            let mut batch = vec![event];
            while let Ok(queued) = events.try_recv() {
                batch.push(queued);
            }
            if batch.len() > 1 {
                info!("Coalesced {} change events into one rebuild", batch.len());
            }

            let result = self.run_build().await;
            drop(guard);

            match result {
                Ok(()) => {
                    let change_type = batch[0].change_type;
                    let files: Vec<String> = batch
                        .iter()
                        .flat_map(|e| e.paths.iter())
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    let reload_event = ReloadEvent::new(change_type, files);
                    let _ = self.reload_tx.send(WebSocketMessage::Reload(reload_event));
                }
                Err(e) => error!("Build failed: {}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::watcher::ChangeType;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::sync::Notify;
    use tokio::time::timeout;

    /// Instrumented rebuild function.
    ///
    /// Every call registers itself as in flight, reports that it started on
    /// `started`, then parks on `gate` until the test releases it with
    /// `Probe::release`.  Time is paused in every test, so nothing here is
    /// timing-dependent: a build finishes exactly when the test says so.
    struct Probe {
        calls: AtomicUsize,
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
        started_tx: mpsc::UnboundedSender<()>,
        gate: Notify,
        /// Zero-based call indices that should return `Err`.
        fail_calls: Vec<usize>,
    }

    impl Probe {
        fn new(fail_calls: Vec<usize>) -> (Arc<Self>, mpsc::UnboundedReceiver<()>) {
            let (started_tx, started_rx) = mpsc::unbounded_channel();
            let probe = Arc::new(Self {
                calls: AtomicUsize::new(0),
                in_flight: AtomicUsize::new(0),
                max_in_flight: AtomicUsize::new(0),
                started_tx,
                gate: Notify::new(),
                fail_calls,
            });
            (probe, started_rx)
        }

        fn rebuild_fn(self: &Arc<Self>) -> AsyncRebuildFn {
            let probe = self.clone();
            Arc::new(move || {
                let probe = probe.clone();
                Box::pin(async move {
                    let now = probe.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    probe.max_in_flight.fetch_max(now, Ordering::SeqCst);
                    let _ = probe.started_tx.send(());

                    probe.gate.notified().await;

                    probe.in_flight.fetch_sub(1, Ordering::SeqCst);
                    let call = probe.calls.fetch_add(1, Ordering::SeqCst);
                    if probe.fail_calls.contains(&call) {
                        Err(format!("boom {}", call))
                    } else {
                        Ok(())
                    }
                })
            })
        }

        /// Let the currently parked build (or the next one to park) finish.
        fn release(&self) {
            self.gate.notify_one();
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }

        fn in_flight(&self) -> usize {
            self.in_flight.load(Ordering::SeqCst)
        }

        fn max_in_flight(&self) -> usize {
            self.max_in_flight.load(Ordering::SeqCst)
        }
    }

    /// Wait for the next build to start. Time is paused, so a build that
    /// never starts fails fast instead of hanging the test.
    async fn wait_started(started: &mut mpsc::UnboundedReceiver<()>) {
        timeout(Duration::from_secs(5), started.recv())
            .await
            .expect("a build should have started")
            .expect("probe dropped");
    }

    /// Assert that no further build starts once the runtime goes idle.
    async fn assert_no_more_builds(started: &mut mpsc::UnboundedReceiver<()>) {
        assert!(
            timeout(Duration::from_secs(5), started.recv())
                .await
                .is_err(),
            "an extra build was started"
        );
    }

    async fn next_message(rx: &mut broadcast::Receiver<WebSocketMessage>) -> WebSocketMessage {
        timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("a message should have been broadcast")
            .expect("broadcast channel closed")
    }

    fn event(path: &str) -> WatchEvent {
        WatchEvent::new(ChangeType::Content, vec![PathBuf::from(path)])
    }

    struct Fixture {
        probe: Arc<Probe>,
        started: mpsc::UnboundedReceiver<()>,
        coordinator: Arc<RebuildCoordinator>,
        events_tx: mpsc::Sender<WatchEvent>,
        reload_rx: broadcast::Receiver<WebSocketMessage>,
        _shutdown_tx: oneshot::Sender<()>,
    }

    /// Spawn a coordinator watch loop driven by an instrumented probe.
    fn spawn_loop(fail_calls: Vec<usize>) -> Fixture {
        let (probe, started) = Probe::new(fail_calls);
        let (reload_tx, reload_rx) = broadcast::channel(16);
        let coordinator = Arc::new(RebuildCoordinator::new(probe.rebuild_fn(), reload_tx));
        let (events_tx, events_rx) = mpsc::channel(64);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let looped = coordinator.clone();
        tokio::spawn(async move { looped.run(events_rx, shutdown_rx).await });

        Fixture {
            probe,
            started,
            coordinator,
            events_tx,
            reload_rx,
            _shutdown_tx: shutdown_tx,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn triggers_during_a_build_never_run_concurrently() {
        let mut fx = spawn_loop(vec![]);

        fx.events_tx.send(event("content/a.md")).await.unwrap();
        wait_started(&mut fx.started).await;
        assert_eq!(fx.probe.in_flight(), 1);

        // Two more triggers arrive while the first build is in flight.
        fx.events_tx.send(event("content/b.md")).await.unwrap();
        fx.events_tx.send(event("content/c.md")).await.unwrap();
        tokio::task::yield_now().await;
        assert_eq!(
            fx.probe.in_flight(),
            1,
            "a second build started while one was in flight"
        );

        fx.probe.release();
        wait_started(&mut fx.started).await;
        assert_eq!(fx.probe.in_flight(), 1);
        fx.probe.release();
        assert_no_more_builds(&mut fx.started).await;

        assert_eq!(fx.probe.max_in_flight(), 1, "builds overlapped");
    }

    #[tokio::test(start_paused = true)]
    async fn events_during_a_build_coalesce_into_one_follow_up() {
        let mut fx = spawn_loop(vec![]);

        fx.events_tx.send(event("content/first.md")).await.unwrap();
        wait_started(&mut fx.started).await;

        // N events arrive while the first build is running.
        let n = 5;
        for i in 0..n {
            fx.events_tx
                .send(event(&format!("content/burst-{i}.md")))
                .await
                .unwrap();
        }

        fx.probe.release();
        wait_started(&mut fx.started).await;
        fx.probe.release();
        assert_no_more_builds(&mut fx.started).await;

        assert_eq!(
            fx.probe.calls(),
            2,
            "1 + {n} events should produce exactly 2 rebuilds"
        );

        // The first reload names the first file; the follow-up names every
        // coalesced file so the browser log still shows what changed.
        match next_message(&mut fx.reload_rx).await {
            WebSocketMessage::Reload(ev) => assert_eq!(ev.files, vec!["content/first.md"]),
            other => panic!("expected Reload, got {other:?}"),
        }
        match next_message(&mut fx.reload_rx).await {
            WebSocketMessage::Reload(ev) => {
                assert_eq!(ev.files.len(), n);
                assert!(ev.files.contains(&"content/burst-0.md".to_string()));
                assert!(ev.files.contains(&format!("content/burst-{}.md", n - 1)));
            }
            other => panic!("expected Reload, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_build_is_broadcast_and_does_not_stop_the_loop() {
        let mut fx = spawn_loop(vec![0]);

        fx.events_tx.send(event("content/bad.md")).await.unwrap();
        wait_started(&mut fx.started).await;
        fx.probe.release();

        match next_message(&mut fx.reload_rx).await {
            WebSocketMessage::Error { message } => {
                assert!(message.contains("Build failed"), "got {message:?}");
                assert!(message.contains("boom 0"), "got {message:?}");
            }
            other => panic!("expected Error, got {other:?}"),
        }

        // The next event must still trigger a build, and succeed normally.
        fx.events_tx.send(event("content/good.md")).await.unwrap();
        wait_started(&mut fx.started).await;
        fx.probe.release();

        match next_message(&mut fx.reload_rx).await {
            WebSocketMessage::Reload(ev) => assert_eq!(ev.files, vec!["content/good.md"]),
            other => panic!("expected Reload, got {other:?}"),
        }
        assert_eq!(fx.probe.calls(), 2);

        // And an explicit build() call still goes through: the lock was not
        // left held by the failure.
        let coordinator = fx.coordinator.clone();
        let direct = tokio::spawn(async move { coordinator.build().await });
        wait_started(&mut fx.started).await;
        fx.probe.release();
        assert!(direct.await.unwrap().is_ok());
        assert_eq!(fx.probe.calls(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn initial_build_and_watcher_event_serialize_at_startup() {
        let mut fx = spawn_loop(vec![]);

        // The initial build and a change event race at startup.
        let coordinator = fx.coordinator.clone();
        let initial = tokio::spawn(async move { coordinator.build().await });
        fx.events_tx.send(event("content/a.md")).await.unwrap();

        wait_started(&mut fx.started).await;
        tokio::task::yield_now().await;
        assert_eq!(
            fx.probe.in_flight(),
            1,
            "initial build and watcher build overlapped"
        );

        fx.probe.release();
        wait_started(&mut fx.started).await;
        assert_eq!(fx.probe.in_flight(), 1);
        fx.probe.release();
        assert_no_more_builds(&mut fx.started).await;

        assert!(initial.await.unwrap().is_ok());
        assert_eq!(fx.probe.calls(), 2);
        assert_eq!(fx.probe.max_in_flight(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn loop_exits_on_shutdown() {
        let (probe, _started) = Probe::new(vec![]);
        let (reload_tx, _) = broadcast::channel(16);
        let coordinator = Arc::new(RebuildCoordinator::new(probe.rebuild_fn(), reload_tx));
        let (_events_tx, events_rx) = mpsc::channel::<WatchEvent>(64);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let handle = tokio::spawn(async move { coordinator.run(events_rx, shutdown_rx).await });
        shutdown_tx.send(()).unwrap();
        timeout(Duration::from_secs(5), handle)
            .await
            .expect("loop should exit on shutdown")
            .unwrap();
        assert_eq!(probe.calls(), 0);
    }
}
