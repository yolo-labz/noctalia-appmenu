# Contract: `FocusSink` trait (Lane A)

**Status:** introduced in v1.0.0
**File:** `bridge/src/focus.rs` (new)
**Consumers:** `bridge/src/main.rs`, `bridge/src/active.rs`
**Implementors:** `bridge/src/niri.rs` (the only one at v1)

## Interface

```rust
#[async_trait::async_trait]
pub trait FocusSink: Send + Sync + 'static {
    /// Runs until the channel is closed or a fatal error occurs.
    /// Sends `Some(FocusEvent)` on focus changes and `None` on focus-cleared.
    /// Resets internal backoff state to floor after a connected session
    /// of at least 30 seconds, so transient compositor restarts do not
    /// compound into multi-second blank-bar gaps.
    async fn run(
        &mut self,
        tx: tokio::sync::watch::Sender<Option<crate::focus::FocusEvent>>,
        cfg: crate::config::Config,
    ) -> anyhow::Result<()>;
}
```

## Companion types

```rust
#[derive(Clone, Debug)]
pub struct FocusEvent {
    pub id: u64,
    pub pid: i32,
    pub app_id: String,
    pub title: String,
    pub timestamp: std::time::Instant,
}

#[derive(Clone, Debug)]
pub enum FocusOp {
    Emit(FocusEvent),
    Clear,
    NoOp,
}
```

## Contract guarantees

1. **Monotonic `timestamp`.** Each implementor must ensure per-source monotonicity; out-of-order events are dropped before send.
2. **Debounce semantics.** Trailing-edge debounce of 75 ms (ADR-0009). Implementor MAY apply the debounce internally OR pass through and let `active.rs` debounce — the trait does not constrain the location.
3. **Backoff reset.** After a connected session of duration ≥ 30 s ends cleanly (peer EOF), the implementor MUST reset reconnect backoff to its floor (default 250 ms). See FR-001.
4. **Ack-path observability.** A successful subscription ack must complete within 5 s of connect, OR the implementor must log a diagnostic and return `Err(_)` so the supervisor retries. See FR-002.
5. **Cancellation safety.** `run` must be `tokio::select!`-cancellable. On cancellation, all sockets / subscriptions must close cleanly before returning.
6. **No panics.** `unwrap` / `expect` only on infallible operations; all D-Bus / IPC failures return typed errors.

## Test contract

- **Unit test for backoff reset** (FR-001 acceptance): a synthetic implementor exercises three EOF cycles of 30+ s each; the test asserts post-third-EOF reconnect attempt fires within 500 ms.
- **Unit test for ack-path** (FR-002 acceptance): a fixture-driven reader exposes a malformed ack response; the test asserts `Err` is returned with a typed `AckParse` variant, not a panic and not a silent loop.

## Breaking-change policy

This trait is private to the `bridge` crate at v1.0.0. The trait surface MAY change in v1.x without a major bump as long as `niri.rs`'s implementor is updated in the same PR. v2 adds Hyprland / Sway sinks under the same trait; major change to the trait shape requires a v2 bump.

## Non-goals

- The trait does **not** model focus-history (which window was previously focused). `active.rs` owns that state.
- The trait does **not** model multi-seat scenarios. niri assumes a single seat; future implementors may extend.
