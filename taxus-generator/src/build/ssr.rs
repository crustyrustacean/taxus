// taxus-generator/src/build/ssr.rs

//! Runtime-agnostic driver for island server-side rendering.
//!
//! Yew's [`ServerRenderer`](yew::ServerRenderer) produces a future, but
//! [`SiteBuilder::build`](crate::build::SiteBuilder::build) is a synchronous
//! API that library consumers call from plain `fn main`, the CLI calls from a
//! `#[tokio::main]` worker, and the dev server calls from `spawn_blocking`.
//! Island SSR therefore has to block on that future from *any* thread state:
//!
//! - **(a)** plain sync code with no tokio runtime at all,
//! - **(b)** inside a `current_thread` runtime (`#[tokio::test]` default),
//! - **(c)** inside a `multi_thread` runtime worker (the CLI build path),
//! - **(d)** inside `spawn_blocking` on a `multi_thread` runtime (the
//!   dev-server rebuild path).
//!
//! [`block_on_ssr`] handles all four (#37) by never blocking on the caller's
//! thread. It runs the future on a fresh scoped OS thread, which has no tokio
//! context of its own, and drives it there with a process-wide
//! `current_thread` runtime. The scoped join keeps the call synchronous.
//!
//! # Rejected alternatives
//!
//! Each of these was tried, or is the obvious thing to reach for, and each
//! panics in at least one of the contexts above:
//!
//! - **`tokio::task::block_in_place(|| Handle::current().block_on(fut))`**
//!   (the previous implementation). `Handle::current()` panics with no
//!   ambient runtime (context a), and `block_in_place` panics on a
//!   `current_thread` runtime (context b). Only (c) and (d) worked, which is
//!   why the CLI was fine and the documented library usage was not.
//! - **`Handle::current().block_on(fut)` alone.** Still panics in (a), and in
//!   (c) `Runtime::block_on`/`Handle::block_on` from inside a runtime context
//!   panics with "Cannot start a runtime from within a runtime".
//! - **A fresh `current_thread` `Runtime` per call** (CHANGELOG 0.1.18).
//!   Fixes (a) but `Runtime::block_on` from within an existing runtime
//!   context (b, c) panics with the same "Cannot start a runtime from within a
//!   runtime" error.
//! - **`Handle::try_current()` and branch.** Works for (a) and (d), but there
//!   is no non-panicking way to block on a future from *inside* a
//!   `current_thread` runtime (b) without a second thread, so a thread is
//!   needed anyway.
//!
//! Spawning one short-lived OS thread per island render costs on the order of
//! tens of microseconds, which is negligible against a site build.

use std::future::Future;
use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

/// Process-wide runtime that drives island SSR futures.
///
/// A `current_thread` runtime is enough: the future produced by
/// `ServerRenderer::render()` only awaits a channel fed by Yew's own SSR
/// worker pool, so there is nothing for extra worker threads to do. It lives
/// in a `OnceLock` so the runtime (and Yew's worker pool behind it) is built
/// once per process rather than once per island.
static SSR_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn ssr_runtime() -> &'static Runtime {
    SSR_RUNTIME.get_or_init(|| {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build the island SSR tokio runtime")
    })
}

/// Block on an island SSR future from any calling context.
///
/// The future is driven to completion on a dedicated scoped thread using the
/// shared [`SSR_RUNTIME`], so this is safe to call whether or not the caller
/// is already inside a tokio runtime, and regardless of that runtime's
/// flavour. See the module docs for why the more obvious approaches panic.
///
/// # Panics
///
/// Propagates a panic from the future itself; never panics because of the
/// caller's runtime state.
pub(crate) fn block_on_ssr<F>(fut: F) -> F::Output
where
    F: Future + Send,
    F::Output: Send,
{
    std::thread::scope(|scope| {
        scope
            .spawn(move || ssr_runtime().block_on(fut))
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use taxus_common::components::counter::{Counter, CounterProps};
    use yew::ServerRenderer;

    /// SSR a `Counter` through `block_on_ssr` and check the initial value
    /// made it into the markup.
    fn render_counter(initial: i32) {
        let html = block_on_ssr(
            ServerRenderer::<Counter>::with_props(move || CounterProps {
                initial,
                class: String::new(),
            })
            .render(),
        );
        assert!(
            html.contains(&format!(r#"<span class="counter-value">{initial}</span>"#)),
            "missing SSR'd initial value {initial}: {html}"
        );
    }

    #[test]
    fn block_on_ssr_without_runtime() {
        render_counter(1);
    }

    #[tokio::test]
    async fn block_on_ssr_inside_current_thread_runtime() {
        render_counter(2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn block_on_ssr_inside_multi_thread_runtime() {
        render_counter(3);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn block_on_ssr_inside_spawn_blocking() {
        tokio::task::spawn_blocking(|| render_counter(4))
            .await
            .expect("spawn_blocking task panicked");
    }

    #[test]
    fn block_on_ssr_from_concurrent_threads() {
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..4)
                .map(|i| s.spawn(move || render_counter(10 + i)))
                .collect();
            for h in handles {
                h.join().expect("SSR thread panicked");
            }
        });
    }

    #[test]
    fn block_on_ssr_returns_future_output() {
        assert_eq!(block_on_ssr(async { 40 + 2 }), 42);
    }

    #[test]
    fn block_on_ssr_propagates_future_panic() {
        let result = std::panic::catch_unwind(|| {
            block_on_ssr(async { panic!("boom") });
        });
        assert!(result.is_err());
    }
}
