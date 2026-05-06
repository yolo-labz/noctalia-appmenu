//! niri-IPC integration: focus-changed events + windows snapshot.
//!
//! On startup we shell out to `niri msg --json windows` once to seed
//! the `winid -> pid` map. We then long-pipe `niri msg --json
//! event-stream` and update the map (and emit focus changes) as events
//! arrive. ADR-0002 + ADR-0005.

use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use serde::de::Deserializer;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;
use tracing::{debug, info, warn};

/// A snapshot of one window as reported by `niri msg --json windows`.
///
/// Some fields (`workspace_id`, `is_focused`) are not currently consumed by
/// the bridge but are kept on the type so downstream debug-printing
/// surfaces useful context. Marked `dead_code`-allowed at the struct
/// level — we control the schema, niri may stop emitting these later.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct NiriWindow {
    /// Stable per-niri-session window identifier.
    pub id: u64,
    /// Wayland app-id (e.g. `kate`, `org.kde.kate`).
    pub app_id: Option<String>,
    /// Window title at snapshot time.
    pub title: Option<String>,
    /// Process ID of the `wl_client` owning the surface.
    pub pid: Option<u32>,
    /// Workspace the window is anchored to.
    pub workspace_id: Option<u64>,
    /// Whether the window had keyboard focus at snapshot time.
    pub is_focused: Option<bool>,
}

/// Niri event-stream variants the bridge cares about.
///
/// Wire format is serde's default *externally-tagged* enum:
/// `{"WindowFocusChanged": {"id": 7}}`. Earlier versions of this file
/// used `#[serde(tag = "type")]` (internally-tagged), which silently
/// dropped EVERY event in production — see ADR-0016 / PR #23 / v0.1.4.
/// Real journal samples exercising this schema live in the unit tests.
///
/// Deserialize is implemented manually because serde's `#[serde(other)]`
/// catch-all only works on internally/adjacently-tagged enums; for the
/// externally-tagged form we need an explicit "fall through to Other on
/// parse failure" so unknown niri variants don't crash the stream.
#[derive(Debug)]
pub enum NiriEvent {
    WindowFocusChanged {
        id: Option<u64>,
    },
    WindowOpenedOrChanged {
        window: NiriWindow,
    },
    WindowClosed {
        id: u64,
    },
    /// Anything else niri emits — workspace/overview/keyboard events
    /// the bridge doesn't currently consume. NOT an error.
    Other,
}

impl<'de> Deserialize<'de> for NiriEvent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Deserialize once into a Value, then try the typed schema. Any
        // parse failure means "variant the bridge doesn't model" → Other.
        // Genuine I/O / non-JSON errors still surface from the caller's
        // `serde_json::from_str` step before we get here.
        // Variant names must match niri's JSON keys verbatim — the
        // shared `Window` prefix is dictated by the wire format, not
        // by us. Hence the `enum_variant_names` allow.
        #[derive(Deserialize)]
        #[allow(clippy::enum_variant_names)]
        enum Typed {
            WindowFocusChanged { id: Option<u64> },
            WindowOpenedOrChanged { window: NiriWindow },
            WindowClosed { id: u64 },
        }

        let value = serde_json::Value::deserialize(deserializer)?;
        match serde_json::from_value::<Typed>(value) {
            Ok(Typed::WindowFocusChanged { id }) => Ok(NiriEvent::WindowFocusChanged { id }),
            Ok(Typed::WindowOpenedOrChanged { window }) => {
                Ok(NiriEvent::WindowOpenedOrChanged { window })
            }
            Ok(Typed::WindowClosed { id }) => Ok(NiriEvent::WindowClosed { id }),
            Err(_) => Ok(NiriEvent::Other),
        }
    }
}

/// What we publish for downstream consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusEvent {
    /// niri's stable window id for this surface.
    pub winid: u64,
    /// PID of the `wl_client` owning the focused surface.
    pub pid: u32,
    /// Wayland app-id for the focused surface.
    pub app_id: String,
    /// Title of the focused surface.
    pub title: String,
}

/// Map operation produced by interpreting one `NiriEvent` against the
/// current `winid -> NiriWindow` cache. Pure — no side effects.
#[derive(Debug, PartialEq, Eq)]
pub enum MapOp {
    /// Replace or insert the window record under the given id.
    Upsert(u64, NiriWindow),
    /// Drop the entry for this id.
    Remove(u64),
    /// Defocused — clear the published focus event.
    DefocusAll,
    /// Focused window record is in cache; emit it.
    FocusEmit(FocusEvent),
    /// Focused id missing from cache; caller should re-snapshot.
    FocusUnknown(u64),
    /// Focused window in cache but has no pid; warn-and-skip.
    FocusNoPid(u64),
    /// Event variant we don't care about (e.g. workspace changes).
    NoOp,
}

/// Pure transducer: interpret one event line + current cache state into
/// a `MapOp`. Extracted from `run()` so we can unit-test the schema-
/// drift behaviour without spawning niri.
///
/// `cache` is read-only; the caller applies the resulting `MapOp` to its
/// own mutable cache. This makes the function trivially testable and
/// independently reorderable.
#[must_use]
pub fn handle_event(event: NiriEvent, cache: &HashMap<u64, NiriWindow>) -> MapOp {
    match event {
        NiriEvent::WindowFocusChanged { id: Some(id) } => match cache.get(&id) {
            Some(win) => match win.pid {
                Some(pid) => MapOp::FocusEmit(FocusEvent {
                    winid: id,
                    pid,
                    app_id: win.app_id.clone().unwrap_or_default(),
                    title: win.title.clone().unwrap_or_default(),
                }),
                None => MapOp::FocusNoPid(id),
            },
            None => MapOp::FocusUnknown(id),
        },
        NiriEvent::WindowFocusChanged { id: None } => MapOp::DefocusAll,
        NiriEvent::WindowOpenedOrChanged { window } => MapOp::Upsert(window.id, window),
        NiriEvent::WindowClosed { id } => MapOp::Remove(id),
        NiriEvent::Other => MapOp::NoOp,
    }
}

/// Long-running task: subscribe to niri's event-stream and forward
/// debounced focus events on `tx`.
///
/// Hard-fails on initial-seed errors (per audit P1 — silent fallback
/// led to "menu invisible" without journalctl evidence). Exits when
/// niri's pipe closes; systemd then restarts the unit.
pub async fn run(tx: watch::Sender<Option<FocusEvent>>, cfg: Config) -> Result<()> {
    let mut by_winid = snapshot_windows(&cfg)
        .await
        .context("seeding niri window map; is `niri` reachable?")?;

    info!(count = by_winid.len(), "seeded niri windows");

    // Long-pipe event stream.
    let mut child = Command::new(&cfg.niri_binary)
        .args(["msg", "--json", "event-stream"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning {} msg event-stream", cfg.niri_binary.display()))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("niri event-stream stdout pipe missing"))?;
    let mut lines = BufReader::new(stdout).lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let event = match serde_json::from_str::<NiriEvent>(line) {
            Ok(e) => e,
            Err(e) => {
                warn!(error=?e, line=%line, "could not parse niri event line");
                continue;
            }
        };
        match handle_event(event, &by_winid) {
            MapOp::FocusEmit(evt) => {
                debug!(?evt, "focus changed");
                let _ = tx.send(Some(evt));
            }
            MapOp::FocusNoPid(id) => {
                warn!(winid = id, "focused window has no pid in our map");
            }
            MapOp::FocusUnknown(id) => {
                warn!(winid = id, "focused window not in cache; resyncing");
                if let Ok(fresh) = snapshot_windows(&cfg).await {
                    by_winid = fresh;
                }
            }
            MapOp::DefocusAll => {
                let _ = tx.send(None);
            }
            MapOp::Upsert(id, window) => {
                by_winid.insert(id, window);
            }
            MapOp::Remove(id) => {
                by_winid.remove(&id);
            }
            MapOp::NoOp => {}
        }
    }

    // niri's pipe closed — usually because niri itself exited or the
    // user logged out of the session. NOT a fatal error; warn-level
    // and let the caller's systemd unit restart us cleanly.
    warn!("niri event-stream ended");
    Err(anyhow!("niri event-stream ended"))
}

async fn snapshot_windows(cfg: &Config) -> Result<HashMap<u64, NiriWindow>> {
    let out = Command::new(&cfg.niri_binary)
        .args(["msg", "--json", "windows"])
        .output()
        .await
        .with_context(|| format!("running {} msg windows", cfg.niri_binary.display()))?;

    if !out.status.success() {
        return Err(anyhow!(
            "niri msg windows exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    let windows: Vec<NiriWindow> =
        serde_json::from_slice(&out.stdout).context("parsing niri msg windows JSON")?;

    Ok(windows.into_iter().map(|w| (w.id, w)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(id: u64, pid: Option<u32>) -> NiriWindow {
        NiriWindow {
            id,
            app_id: Some(format!("app-{id}")),
            title: Some(format!("title-{id}")),
            pid,
            workspace_id: Some(1),
            is_focused: Some(false),
        }
    }

    #[test]
    fn focus_known_window_emits_event() {
        let mut cache = HashMap::new();
        cache.insert(7, win(7, Some(123)));
        let op = handle_event(NiriEvent::WindowFocusChanged { id: Some(7) }, &cache);
        match op {
            MapOp::FocusEmit(evt) => {
                assert_eq!(evt.pid, 123);
                assert_eq!(evt.app_id, "app-7");
                assert_eq!(evt.winid, 7);
            }
            other => panic!("expected FocusEmit, got {other:?}"),
        }
    }

    #[test]
    fn focus_unknown_window_signals_resync() {
        let cache = HashMap::new();
        assert_eq!(
            handle_event(NiriEvent::WindowFocusChanged { id: Some(99) }, &cache),
            MapOp::FocusUnknown(99)
        );
    }

    #[test]
    fn focus_window_without_pid_warns() {
        let mut cache = HashMap::new();
        cache.insert(3, win(3, None));
        assert_eq!(
            handle_event(NiriEvent::WindowFocusChanged { id: Some(3) }, &cache),
            MapOp::FocusNoPid(3)
        );
    }

    #[test]
    fn focus_none_means_defocus_all() {
        let cache = HashMap::new();
        assert_eq!(
            handle_event(NiriEvent::WindowFocusChanged { id: None }, &cache),
            MapOp::DefocusAll
        );
    }

    #[test]
    fn opened_or_changed_upserts() {
        let cache = HashMap::new();
        let w = win(11, Some(456));
        let op = handle_event(NiriEvent::WindowOpenedOrChanged { window: w }, &cache);
        match op {
            MapOp::Upsert(id, _) => assert_eq!(id, 11),
            other => panic!("expected Upsert, got {other:?}"),
        }
    }

    #[test]
    fn closed_removes() {
        let cache = HashMap::new();
        assert_eq!(
            handle_event(NiriEvent::WindowClosed { id: 5 }, &cache),
            MapOp::Remove(5)
        );
    }

    #[test]
    fn unknown_event_is_noop() {
        let cache = HashMap::new();
        assert_eq!(handle_event(NiriEvent::Other, &cache), MapOp::NoOp);
    }

    // Wire-format regression tests — every sample below was captured
    // from `niri msg --json event-stream` on Pedro's desktop running
    // niri 26.04. v0.1.0..v0.1.3 used internally-tagged form and
    // silently dropped 100% of these — covered by PR #23 / ADR-0016.

    #[test]
    fn parses_window_focus_changed_with_id() {
        let line = r#"{"WindowFocusChanged":{"id":7}}"#;
        let evt: NiriEvent = serde_json::from_str(line).expect("must parse");
        match evt {
            NiriEvent::WindowFocusChanged { id: Some(7) } => {}
            other => panic!("expected WindowFocusChanged{{id:7}}, got {other:?}"),
        }
    }

    #[test]
    fn parses_window_focus_changed_with_null() {
        let line = r#"{"WindowFocusChanged":{"id":null}}"#;
        let evt: NiriEvent = serde_json::from_str(line).expect("must parse");
        assert!(matches!(evt, NiriEvent::WindowFocusChanged { id: None }));
    }

    #[test]
    fn parses_window_closed() {
        let line = r#"{"WindowClosed":{"id":42}}"#;
        let evt: NiriEvent = serde_json::from_str(line).expect("must parse");
        assert!(matches!(evt, NiriEvent::WindowClosed { id: 42 }));
    }

    #[test]
    fn parses_window_opened_or_changed() {
        let line = r#"{"WindowOpenedOrChanged":{"window":{"id":3,"app_id":"firefox","title":"hi","pid":1234,"workspace_id":1,"is_focused":true}}}"#;
        let evt: NiriEvent = serde_json::from_str(line).expect("must parse");
        match evt {
            NiriEvent::WindowOpenedOrChanged { window } => {
                assert_eq!(window.id, 3);
                assert_eq!(window.app_id.as_deref(), Some("firefox"));
                assert_eq!(window.pid, Some(1234));
            }
            other => panic!("expected WindowOpenedOrChanged, got {other:?}"),
        }
    }

    #[test]
    fn unknown_variants_become_other_not_error() {
        // Real samples from journalctl that niri emits but bridge ignores.
        for line in &[
            r#"{"OverviewOpenedOrClosed":{"is_open":false}}"#,
            r#"{"ConfigLoaded":{"failed":false}}"#,
            r#"{"WorkspacesChanged":{"workspaces":[]}}"#,
            r#"{"WindowsChanged":{"windows":[]}}"#,
            r#"{"KeyboardLayoutsChanged":{"keyboard_layouts":{"names":[],"current_idx":0}}}"#,
        ] {
            let evt: NiriEvent =
                serde_json::from_str(line).unwrap_or_else(|e| panic!("must parse {line}: {e}"));
            assert!(matches!(evt, NiriEvent::Other), "line {line} -> {evt:?}");
        }
    }

    #[test]
    fn internally_tagged_form_falls_through_to_other() {
        // The v0.1.0..v0.1.3 schema (`{"type": "...", "id": ...}`) is
        // not a real niri wire format — but if some future niri build
        // emits one, we must not crash. Falls into Other (warn-and-skip
        // path), not an error.
        let line = r#"{"type":"WindowFocusChanged","id":7}"#;
        let evt: NiriEvent = serde_json::from_str(line).expect("must parse to Other");
        assert!(matches!(evt, NiriEvent::Other));
    }
}
