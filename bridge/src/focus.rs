//! Focus tracker abstraction (FR-003 of spec 005-bridge-completion).
//!
//! Per ADR-0024 the bridge's substrate is AT-SPI; per constitution
//! principle I, v1.0.0 ships niri as the only compositor.
//!
//! This module is the **abstraction door** for future compositors
//! (Hyprland / Sway / KWin / COSMIC). At v1.0.0 the only implementor
//! is [`crate::niri::NiriFocusSink`]; downstream consumers (`active.rs`,
//! `main.rs`) depend on the types here, NOT on `crate::niri::*` —
//! that way swapping or adding implementors does not ripple beyond
//! `niri.rs`.
//!
//! ## Contract (per `specs/004-project-completion/contracts/focus-sink-trait.md`)
//!
//! 1. Implementors emit `Some(FocusEvent)` on focus-change and
//!    `None` to clear focus.
//! 2. Reconnect backoff resets to the floor after a connected
//!    session of ≥ 30 s (FR-001).
//! 3. Ack-path parse failures surface as typed `Err` — never silent
//!    backoff (FR-002).
//! 4. `run` is `tokio::select!`-cancellation safe and returns
//!    `Ok(())` only when `tx` is closed.
//!
//! ## Why a boxed future
//!
//! Rust 1.81 (the bridge's MSRV) does not yet stabilise
//! return-type-notation (`T::method(..): Send`) for `async fn` in
//! traits. The boxed-future return + `Self: Sized` receiver is the
//! smallest stable equivalent and keeps the trait dep-free (no
//! `async-trait` macro).
//!
//! ## Why `Self: Sized`
//!
//! `run` consumes `self` — the focus sink is meant to be spawned
//! once and live for the bridge's lifetime. Trait-object dispatch
//! is intentionally *not* a goal; `main.rs` instantiates one
//! concrete implementor and calls `run` directly.

use crate::config::Config;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::watch;

/// What we publish for downstream consumers (`active.rs`).
///
/// `pid` is the wl_client owning the focused surface — the AT-SPI
/// walker uses it to find the matching accessible application. niri
/// reports PID as `i32` but the bridge has long used `u32` end-to-end;
/// implementors cast at the focus-source boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusEvent {
    /// Stable per-session window identifier from the compositor.
    pub winid: u64,
    /// Process ID of the wl_client owning the focused surface.
    pub pid: u32,
    /// Wayland app-id (or compositor-equivalent string) for the
    /// focused surface. Empty when the compositor reports no app-id.
    pub app_id: String,
    /// Title of the focused surface at focus time. Empty when the
    /// compositor reports no title.
    pub title: String,
}

/// Decision produced by an implementor's pure focus-detection
/// transducer. Exposed so unit tests can drive focus logic without
/// standing up a real compositor IPC socket.
#[derive(Debug, PartialEq, Eq)]
pub enum FocusOp {
    /// Emit this focus event downstream.
    Emit(FocusEvent),
    /// Caller should clear the published focus (defocus / empty
    /// workspace / focus moved to a non-window surface).
    Defocus,
    /// No focus-relevant change in this event.
    NoChange,
    /// Focused id is set but we have no record of it in the
    /// implementor's state cache (stale event ordering — niri's docs
    /// warn of cross-event inconsistency). Caller logs + skips; will
    /// resync on the next surface-state event.
    UnknownWindow(u64),
}

/// Boxed future returned from [`FocusSink::run`]. Boxing keeps the
/// trait stable-Rust-compatible (no return-type-notation needed).
pub type FocusRunFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;

/// Long-running compositor focus tracker.
///
/// At v1.0.0 the only implementor is [`crate::niri::NiriFocusSink`].
/// Future compositors (Hyprland, Sway, KWin, COSMIC) plug in here
/// without churning `active.rs` or `main.rs` — see the module-level
/// docs for the abstraction-door rationale.
pub trait FocusSink: Send + 'static {
    /// Run the focus tracker until `tx` is closed or a fatal error
    /// occurs. The implementor MUST honour the contract in the
    /// module-level docs (backoff reset, ack-path observability,
    /// cancellation safety, no panics).
    fn run(self, tx: watch::Sender<Option<FocusEvent>>, cfg: Config) -> FocusRunFuture
    where
        Self: Sized;
}
