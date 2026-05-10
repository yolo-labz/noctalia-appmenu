//! niri-IPC integration: focus detection over the niri Unix socket.
//!
//! ## v0.3.0-alpha.20 — `niri-ipc::Socket` adoption (PR #54)
//!
//! Earlier versions shelled out to `niri msg --json event-stream` and
//! parsed the line-delimited JSON ourselves. That worked but coupled
//! the bridge to a specific `niri` binary on PATH, with three failure
//! modes:
//!
//!   1. `niri-25.11` client crashed on `niri-26.04` server because
//!      *its own* serde rejected the new `CastsChanged` variant. The
//!      bridge then ran into pipe-close + non-zero exit and the only
//!      recovery was respawn-loop with backoff (PR #46).
//!   2. New niri event variants (compositor evolution) silently
//!      dropped via our `Other` catch-all — fine, but we paid the
//!      cost of maintaining a hand-rolled `enum NiriEvent` +
//!      manual `Deserialize` impl.
//!   3. Subprocess plumbing (kill_on_drop, stderr redirect, child
//!      exit status interpretation) was load-bearing for restarts.
//!
//! Adopting the `niri-ipc::socket` API plus `niri-ipc::EventStreamState`
//! eliminates all three: we connect to the niri Unix socket directly
//! (no separate client binary), the canonical event types come with
//! the crate (forward-compat catch on parse failure), and
//! `EventStreamState` maintains the windows/workspaces/focus map for
//! us so the bridge no longer has to.
//!
//! Key invariants preserved:
//!
//!   * Public API (`run` task + `FocusEvent` payload) unchanged —
//!     downstream `proxy.rs` and `active.rs` continue to consume
//!     focus events on the same `watch::Sender<Option<FocusEvent>>`.
//!   * Outer respawn-with-backoff loop survives. Reasons to reconnect
//!     differ (socket close instead of subprocess exit) but the
//!     resilience contract is identical.
//!   * Forward-compat: unknown event variants emitted by a future
//!     niri version are logged + skipped, not fatal.
//!
//! ADR-0002 (focus detection) and ADR-0005 (event-stream subscription)
//! still apply at the protocol level.

use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use niri_ipc::socket::SOCKET_PATH_ENV;
use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Event, Reply, Request, Response, Window};
use std::ffi::OsString;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::watch;
use tracing::{debug, info, warn};

/// What we publish for downstream consumers (registrar / proxy).
///
/// `pid` is the wl_client owning the focused surface — used by AT-SPI
/// walker to find the matching accessible application. niri reports
/// PID as `i32` but the bridge has long used `u32` everywhere; we cast
/// at the boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusEvent {
    /// niri's stable per-session window identifier.
    pub winid: u64,
    /// Process ID of the wl_client owning the focused surface.
    pub pid: u32,
    /// Wayland app-id for the focused surface.
    pub app_id: String,
    /// Title of the focused surface at focus time.
    pub title: String,
}

/// Pure transducer: given a parsed niri event, the maintained state,
/// and the previously-emitted focus id, decide whether to emit a new
/// focus event, signal defocus, or do nothing.
///
/// Extracted so tests can drive the focus-detection logic without
/// connecting a niri socket. The caller is responsible for applying
/// the event to `state` (`state.apply(event.clone())`) BEFORE calling
/// this function — focus detection reads from the post-apply state.
#[derive(Debug, PartialEq, Eq)]
pub enum FocusOp {
    /// Emit this focus event downstream.
    Emit(FocusEvent),
    /// Caller should clear the published focus.
    Defocus,
    /// No focus-relevant change.
    NoChange,
    /// Focused id is set but we have no record of it in `state.windows`
    /// (stale event ordering — niri's docs warn of cross-event
    /// inconsistency). Caller logs + skips; will resync on the next
    /// `WindowOpenedOrChanged`.
    UnknownWindow(u64),
}

/// Inspect a freshly-applied event and decide what to publish.
#[must_use]
pub fn detect_focus_change(event: &Event, state: &EventStreamState) -> FocusOp {
    match event {
        Event::WindowFocusChanged { id: None } => FocusOp::Defocus,
        Event::WindowFocusChanged { id: Some(id) } => emit_for(*id, state),
        // niri emits a single `WindowsChanged` at stream start with the
        // initial window list. If one of them is focused, seed the
        // bridge so a restart doesn't blank the menu strip until the
        // user alt-tabs (codex P0 #2 behaviour preserved from
        // pre-refactor `run_once`).
        Event::WindowsChanged { windows } => windows
            .iter()
            .find(|w| w.is_focused)
            .map(|w| emit_for(w.id, state))
            .unwrap_or(FocusOp::NoChange),
        // A newly-opened or already-changed window can be focused at
        // emit time (e.g. open-and-focus IPC). Emit on its is_focused
        // bit; otherwise fall through.
        Event::WindowOpenedOrChanged { window } if window.is_focused => emit_for(window.id, state),
        // All other events (workspace, overview, casts, keyboard
        // layout, ...) do not affect focus. The state machine has
        // already absorbed them via `state.apply`.
        _ => FocusOp::NoChange,
    }
}

fn emit_for(id: u64, state: &EventStreamState) -> FocusOp {
    let Some(window) = state.windows.windows.get(&id) else {
        return FocusOp::UnknownWindow(id);
    };
    let Some(pid) = window.pid else {
        // niri reports None for windows opened via xdg-desktop-portal-gnome.
        // Without a pid we can't match an AT-SPI accessible. Skip.
        return FocusOp::NoChange;
    };
    FocusOp::Emit(focus_event(id, pid, window))
}

fn focus_event(id: u64, pid: i32, window: &Window) -> FocusEvent {
    FocusEvent {
        winid: id,
        // niri reports PID as i32 but PIDs are positive in practice
        // and our downstream consumers use u32. Cast preserves the
        // bit pattern; on Linux PID_MAX_LIMIT is 4_194_304 (well
        // within u32 range).
        pid: pid as u32,
        app_id: window.app_id.clone().unwrap_or_default(),
        title: window.title.clone().unwrap_or_default(),
    }
}

/// Long-running task: subscribe to niri's event stream and forward
/// focus events on `tx`.
///
/// Outer loop reconnects on socket close / read error with exponential
/// backoff. Returns `Ok(())` only when `tx` is closed (graceful
/// shutdown — caller dropped the receiver). The `cfg` parameter is
/// retained for API compatibility and future per-config options;
/// niri socket path is read from the `NIRI_SOCKET` env var.
pub async fn run(tx: watch::Sender<Option<FocusEvent>>, _cfg: Config) -> Result<()> {
    let mut backoff = std::time::Duration::from_millis(200);
    const BACKOFF_MAX: std::time::Duration = std::time::Duration::from_secs(30);

    loop {
        if tx.is_closed() {
            return Ok(());
        }
        match run_once(&tx).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                warn!(
                    error = ?e,
                    backoff_ms = backoff.as_millis(),
                    "niri socket session ended; reconnecting after backoff"
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(BACKOFF_MAX);
            }
        }
    }
}

/// Single niri socket session: connect → request EventStream → read
/// events until socket close or shutdown signal. Any non-graceful exit
/// returns `Err` so the outer loop reconnects.
async fn run_once(tx: &watch::Sender<Option<FocusEvent>>) -> Result<()> {
    let socket_path: OsString = std::env::var_os(SOCKET_PATH_ENV).ok_or_else(|| {
        anyhow!(
            "{SOCKET_PATH_ENV} is not set; bridge must run inside a niri session \
             (or override via systemd Environment=)"
        )
    })?;

    let stream = UnixStream::connect(&socket_path)
        .await
        .with_context(|| format!("connecting to niri socket at {:?}", socket_path))?;

    let (rd, mut wr) = stream.into_split();
    let mut rd = BufReader::new(rd);

    // Subscribe to the event stream.
    let req = serde_json::to_string(&Request::EventStream)
        .context("serialising EventStream request")?
        + "\n";
    wr.write_all(req.as_bytes())
        .await
        .context("writing EventStream request to niri socket")?;
    wr.shutdown()
        .await
        .context("closing write half of niri socket")?;

    // First reply is the ack — niri responds with `Ok(Response::Handled)`
    // to confirm subscription. After that the connection becomes a
    // one-way stream of newline-delimited `Event` JSON objects.
    let mut buf = String::new();
    let n = rd
        .read_line(&mut buf)
        .await
        .context("reading EventStream ack from niri")?;
    if n == 0 {
        return Err(anyhow!("niri closed socket before EventStream ack"));
    }
    let reply: Reply = serde_json::from_str(buf.trim())
        .with_context(|| format!("parsing EventStream ack: {:?}", buf.trim()))?;
    match reply {
        Ok(Response::Handled) => {}
        Ok(other) => {
            return Err(anyhow!(
                "unexpected EventStream ack response from niri: {:?}",
                other
            ));
        }
        Err(msg) => return Err(anyhow!("niri rejected EventStream subscription: {msg}")),
    }

    info!("subscribed to niri event stream");

    drive_events(&mut rd, tx).await
}

/// Pure event-driving loop: read newline-delimited niri-ipc `Event`
/// JSON from `rd`, apply each to a fresh `EventStreamState`, run the
/// `detect_focus_change` transducer, and send `FocusEvent`s on `tx`.
///
/// Extracted from `run_once` so unit tests can replay captured event
/// logs against the loop without standing up an actual niri socket.
/// The function is generic over any `AsyncBufRead + Unpin` so a
/// `tokio::io::BufReader<UnixStream>` (production), a
/// `tokio::io::BufReader<tokio::io::DuplexStream>` (in-process pipe
/// test), or `std::io::Cursor<Vec<u8>>` wrapped via `tokio_util::io::StreamReader`
/// (file-fixture replay) all drop in.
///
/// Returns `Ok(())` only when `tx` is closed (graceful shutdown).
/// EOF on the read stream surfaces as `Err` so the outer reconnect
/// loop respawns. Per Swarm G of the v3 best-practices synthesis,
/// having this function callable from tests is the single highest-
/// leverage refactor for catching the next compositor IPC drift
/// before it ships — fixture-replay tests live alongside the
/// existing wire-format tests in `mod tests` below.
pub async fn drive_events<R>(rd: &mut R, tx: &watch::Sender<Option<FocusEvent>>) -> Result<()>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    drive_events_with(
        rd,
        |evt| {
            let _ = tx.send(evt);
        },
        || tx.is_closed(),
    )
    .await
}

/// Generic event-loop core. Calls `publish(Option<FocusEvent>)` on
/// every transition (Some on focus change, None on defocus) and
/// returns when `is_closed()` reports true (graceful shutdown) or
/// the read stream EOFs (Err — caller should reconnect).
///
/// The `watch::Sender`-using `drive_events` wrapper is the production
/// caller. Tests use this directly with a `Vec<Option<FocusEvent>>`-
/// pushing closure to capture every transition without `watch`'s
/// "latest only" coalescing semantics.
pub async fn drive_events_with<R, F, C>(rd: &mut R, mut publish: F, mut is_closed: C) -> Result<()>
where
    R: tokio::io::AsyncBufRead + Unpin,
    F: FnMut(Option<FocusEvent>),
    C: FnMut() -> bool,
{
    let mut state = EventStreamState::default();
    let mut buf = String::new();

    loop {
        if is_closed() {
            return Ok(());
        }
        buf.clear();
        let n = rd
            .read_line(&mut buf)
            .await
            .context("reading event from niri stream")?;
        if n == 0 {
            return Err(anyhow!("niri event-stream closed"));
        }

        let line = buf.trim();
        if line.is_empty() {
            continue;
        }

        let event: Event = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                warn!(
                    error = ?e,
                    line = %line,
                    "could not parse niri event line; skipping (likely a niri version newer than the bridge's pinned niri-ipc crate)"
                );
                continue;
            }
        };

        let _ignored = state.apply(event.clone());

        match detect_focus_change(&event, &state) {
            FocusOp::Emit(evt) => {
                debug!(?evt, "focus changed");
                publish(Some(evt));
            }
            FocusOp::Defocus => {
                debug!("defocus");
                publish(None);
            }
            FocusOp::UnknownWindow(id) => {
                warn!(
                    winid = id,
                    "focused window not in state.windows; awaiting WindowOpenedOrChanged"
                );
            }
            FocusOp::NoChange => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use niri_ipc::Window;

    fn window(id: u64, pid: Option<i32>, focused: bool) -> Window {
        Window {
            id,
            app_id: Some(format!("app-{id}")),
            title: Some(format!("title-{id}")),
            pid,
            workspace_id: Some(1),
            is_focused: focused,
            is_floating: false,
            is_urgent: false,
            layout: niri_ipc::WindowLayout {
                pos_in_scrolling_layout: None,
                tile_size: (0.0, 0.0),
                window_size: (0, 0),
                tile_pos_in_workspace_view: None,
                window_offset_in_tile: (0.0, 0.0),
            },
            focus_timestamp: None,
        }
    }

    fn state_with(windows: Vec<Window>) -> EventStreamState {
        let mut state = EventStreamState::default();
        // Apply WindowsChanged with the seed list so state.windows is
        // populated. Mirrors how niri does it on stream start.
        let evt = Event::WindowsChanged { windows };
        let _ = state.apply(evt);
        state
    }

    #[test]
    fn focus_known_window_emits_event() {
        let state = state_with(vec![window(7, Some(123), false)]);
        let evt = Event::WindowFocusChanged { id: Some(7) };
        match detect_focus_change(&evt, &state) {
            FocusOp::Emit(focus) => {
                assert_eq!(focus.pid, 123);
                assert_eq!(focus.app_id, "app-7");
                assert_eq!(focus.winid, 7);
            }
            other => panic!("expected Emit, got {other:?}"),
        }
    }

    #[test]
    fn focus_unknown_window_signals_unknown() {
        let state = EventStreamState::default();
        let evt = Event::WindowFocusChanged { id: Some(99) };
        assert_eq!(
            detect_focus_change(&evt, &state),
            FocusOp::UnknownWindow(99)
        );
    }

    #[test]
    fn focus_window_without_pid_is_no_change() {
        let state = state_with(vec![window(3, None, false)]);
        let evt = Event::WindowFocusChanged { id: Some(3) };
        assert_eq!(detect_focus_change(&evt, &state), FocusOp::NoChange);
    }

    #[test]
    fn focus_none_means_defocus() {
        let state = EventStreamState::default();
        let evt = Event::WindowFocusChanged { id: None };
        assert_eq!(detect_focus_change(&evt, &state), FocusOp::Defocus);
    }

    #[test]
    fn windows_changed_emits_initial_focus() {
        let mut state = EventStreamState::default();
        let evt = Event::WindowsChanged {
            windows: vec![
                window(1, Some(100), false),
                window(2, Some(200), true),
                window(3, Some(300), false),
            ],
        };
        let _ignored = state.apply(evt.clone());
        match detect_focus_change(&evt, &state) {
            FocusOp::Emit(focus) => {
                assert_eq!(focus.winid, 2);
                assert_eq!(focus.pid, 200);
            }
            other => panic!("expected initial focus emit, got {other:?}"),
        }
    }

    #[test]
    fn windows_changed_with_no_focused_is_no_change() {
        let evt = Event::WindowsChanged {
            windows: vec![window(1, Some(100), false)],
        };
        let mut state = EventStreamState::default();
        let _ = state.apply(evt.clone());
        assert_eq!(detect_focus_change(&evt, &state), FocusOp::NoChange);
    }

    #[test]
    fn opened_or_changed_with_focus_emits() {
        let w = window(11, Some(456), true);
        let evt = Event::WindowOpenedOrChanged { window: w.clone() };
        let mut state = EventStreamState::default();
        let _ = state.apply(evt.clone());
        match detect_focus_change(&evt, &state) {
            FocusOp::Emit(focus) => assert_eq!(focus.winid, 11),
            other => panic!("expected Emit, got {other:?}"),
        }
    }

    #[test]
    fn opened_or_changed_without_focus_is_no_change() {
        let w = window(12, Some(789), false);
        let evt = Event::WindowOpenedOrChanged { window: w };
        let mut state = EventStreamState::default();
        let _ = state.apply(evt.clone());
        assert_eq!(detect_focus_change(&evt, &state), FocusOp::NoChange);
    }

    #[test]
    fn closed_does_not_affect_focus_decision() {
        let evt = Event::WindowClosed { id: 5 };
        let state = EventStreamState::default();
        assert_eq!(detect_focus_change(&evt, &state), FocusOp::NoChange);
    }

    #[test]
    fn workspace_event_is_no_change() {
        let evt = Event::WorkspacesChanged { workspaces: vec![] };
        let state = EventStreamState::default();
        assert_eq!(detect_focus_change(&evt, &state), FocusOp::NoChange);
    }

    // Wire-format regression tests — verifies niri-ipc 26.4 parses the
    // event lines we used to handle by hand. If a future niri changes
    // the wire format, these tests fail loudly instead of silently
    // dropping.

    #[test]
    fn parses_window_focus_changed_with_id() {
        let line = r#"{"WindowFocusChanged":{"id":7}}"#;
        let evt: Event = serde_json::from_str(line).expect("must parse");
        assert!(matches!(evt, Event::WindowFocusChanged { id: Some(7) }));
    }

    #[test]
    fn parses_window_focus_changed_with_null() {
        let line = r#"{"WindowFocusChanged":{"id":null}}"#;
        let evt: Event = serde_json::from_str(line).expect("must parse");
        assert!(matches!(evt, Event::WindowFocusChanged { id: None }));
    }

    #[test]
    fn parses_window_closed() {
        let line = r#"{"WindowClosed":{"id":42}}"#;
        let evt: Event = serde_json::from_str(line).expect("must parse");
        assert!(matches!(evt, Event::WindowClosed { id: 42 }));
    }

    // ── Fixture-replay tests (PR — adopts Swarm G refactor 2) ────────
    //
    // Captures from a real `niri msg --json event-stream` session can
    // now be replayed against `drive_events` to verify the focus-
    // detection pipeline end-to-end without spinning up a niri
    // compositor. Each fixture is a sequence of newline-delimited
    // niri-ipc Event JSON objects; the test wraps it in a Cursor +
    // tokio BufReader and drives `drive_events` until EOF.
    //
    // Two purposes:
    //   1. Regression sentinel: when niri 27.x adds variants the
    //      pinned crate doesn't know, replay + watch for the warn-
    //      and-skip path firing instead of the full pipeline failing.
    //   2. Correctness oracle: verify that for a recorded "focus
    //      Firefox → focus ghostty → close ghostty" trace, the
    //      bridge emits exactly the FocusEvents the real session
    //      produced.

    use std::cell::RefCell;
    use std::rc::Rc;
    use tokio::io::BufReader as TokioBufReader;

    /// Drive `drive_events_with` against an in-memory event log,
    /// pushing every FocusEvent transition into a Vec and returning
    /// the list. Uses a closure-publishing variant that captures
    /// every transition (the production `watch::Sender` path coalesces
    /// rapid sends, so it's unsuitable for replay-test capture).
    async fn replay(events_jsonl: &str) -> Vec<Option<FocusEvent>> {
        let collected: Rc<RefCell<Vec<Option<FocusEvent>>>> = Rc::new(RefCell::new(Vec::new()));
        let collected_pub = collected.clone();

        let cursor = std::io::Cursor::new(events_jsonl.as_bytes().to_vec());
        let mut rd = TokioBufReader::new(cursor);
        // drive_events_with returns Err on EOF; that's expected here.
        let _ = drive_events_with(
            &mut rd,
            move |evt| collected_pub.borrow_mut().push(evt),
            || false,
        )
        .await;
        Rc::try_unwrap(collected)
            .expect("Rc still held")
            .into_inner()
    }

    #[tokio::test]
    async fn replay_focus_firefox_then_ghostty() {
        // Capture: niri starts with two windows, Firefox initially
        // focused, then user alt-tabs to ghostty, then closes ghostty.
        let log = concat!(
            r#"{"WindowsChanged":{"windows":[{"id":1,"app_id":"firefox","title":"Inbox","pid":2001,"workspace_id":1,"is_focused":true,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":null,"tile_size":[0.0,0.0],"window_size":[0,0],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[0.0,0.0]},"focus_timestamp":null},{"id":2,"app_id":"ghostty","title":"~","pid":2002,"workspace_id":2,"is_focused":false,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":null,"tile_size":[0.0,0.0],"window_size":[0,0],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[0.0,0.0]},"focus_timestamp":null}]}}"#,
            "\n",
            r#"{"WindowFocusChanged":{"id":2}}"#,
            "\n",
            r#"{"WindowClosed":{"id":2}}"#,
            "\n",
            r#"{"WindowFocusChanged":{"id":1}}"#,
            "\n",
        );
        let observed = replay(log).await;

        // We expect three Some(FocusEvent) emits in order: Firefox
        // (from initial WindowsChanged seed), ghostty (alt-tab), then
        // back to Firefox after ghostty closes. WindowClosed itself
        // does not synthesise a focus change in our pipeline; the
        // niri compositor sends a follow-up WindowFocusChanged.
        let pids: Vec<u32> = observed
            .iter()
            .filter_map(|o| o.as_ref().map(|f| f.pid))
            .collect();
        assert_eq!(
            pids,
            vec![2001, 2002, 2001],
            "expected focus emits for Firefox→ghostty→Firefox; observed {observed:?}"
        );
    }

    #[tokio::test]
    async fn replay_handles_unknown_event_variants() {
        // niri 27.x might add a variant like "FoobarChanged"; our
        // serde_json parse fails for it, the loop logs and skips,
        // and the next valid event still drives a focus emit.
        let log = concat!(
            r#"{"WindowsChanged":{"windows":[{"id":7,"app_id":"x","title":"t","pid":111,"workspace_id":1,"is_focused":true,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":null,"tile_size":[0.0,0.0],"window_size":[0,0],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[0.0,0.0]},"focus_timestamp":null}]}}"#,
            "\n",
            r#"{"FoobarChanged":{"foo":"bar"}}"#,
            "\n",
            r#"{"WindowFocusChanged":{"id":null}}"#,
            "\n",
        );
        let observed = replay(log).await;
        // Initial seed = Some(pid 111); then defocus = None. Skipped
        // FoobarChanged contributes no entry.
        assert_eq!(observed.len(), 2);
        assert_eq!(observed[0].as_ref().map(|f| f.pid), Some(111));
        assert_eq!(observed[1].as_ref().map(|f| f.pid), None);
    }

    #[tokio::test]
    async fn replay_handles_window_without_pid() {
        // niri reports None for windows opened via xdg-desktop-portal-gnome.
        // Our pipeline should NOT emit a focus event for them.
        let log = concat!(
            r#"{"WindowsChanged":{"windows":[{"id":3,"app_id":"portal","title":"chooser","pid":null,"workspace_id":1,"is_focused":true,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":null,"tile_size":[0.0,0.0],"window_size":[0,0],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[0.0,0.0]},"focus_timestamp":null}]}}"#,
            "\n",
        );
        let observed = replay(log).await;
        assert_eq!(
            observed.len(),
            0,
            "no emit for pidless window; got {observed:?}"
        );
    }

    #[tokio::test]
    async fn replay_empty_lines_are_skipped() {
        // Defensive: blank lines in the stream (rare niri quirk or
        // newline-only chunks) must not crash the loop.
        let log = concat!(
            "\n",
            r#"{"WindowFocusChanged":{"id":null}}"#,
            "\n",
            "\n",
            "\n",
        );
        let observed = replay(log).await;
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0], None);
    }
}
