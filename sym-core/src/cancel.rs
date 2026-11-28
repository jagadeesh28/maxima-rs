//! Cooperative compute cancellation token (ADR-0014).
//!
//! Provides a lightweight [`CancelToken`] backed by `Arc<AtomicBool>` and a
//! thread-local scope guard [`with_cancel_token`] so e-graph saturation and
//! procedural CAS loops can abort promptly when a request times out.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Cooperative cancellation token polled by e-graph saturation and CAS loops.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    /// Create a new uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Signal cancellation to any worker polling this token.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Returns `true` if cancellation has been requested.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Borrow the underlying atomic flag.
    pub fn as_arc(&self) -> &Arc<AtomicBool> {
        &self.cancelled
    }
}

thread_local! {
    static ACTIVE_CANCEL_TOKEN: RefCell<Option<CancelToken>> = const { RefCell::new(None) };
}

struct ResetGuard {
    prev: Option<CancelToken>,
}

impl Drop for ResetGuard {
    fn drop(&mut self) {
        let prev = self.prev.take();
        ACTIVE_CANCEL_TOKEN.with(|slot| {
            *slot.borrow_mut() = prev;
        });
    }
}

/// Run `f` with `token` installed as the current thread's active cancellation token.
pub fn with_cancel_token<T>(token: &CancelToken, f: impl FnOnce() -> T) -> T {
    let prev = ACTIVE_CANCEL_TOKEN.with(|slot| slot.borrow_mut().replace(token.clone()));
    let _guard = ResetGuard { prev };
    f()
}

/// Returns `true` if the current thread's active [`CancelToken`] has been cancelled.
#[inline]
pub fn is_cancelled() -> bool {
    ACTIVE_CANCEL_TOKEN.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(CancelToken::is_cancelled)
    })
}
