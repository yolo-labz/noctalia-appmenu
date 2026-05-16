//! Owns `org.noctalia.AppMenu` on the session bus and exposes
//! `/org/noctalia/AppMenu/Active` with properties:
//!
//!   - `busName`     : string
//!   - `objectPath`  : string
//!   - `appId`       : string
//!   - `title`       : string
//!
//! The QML widget binds to these. Currently the QML widget then
//! independently attaches a Quickshell `DBusMenuHandle` to
//! `(busName, objectPath)`.
//!
//! Future work (ADR-0007 second-half): mirror the upstream `DBusMenu`
//! itself under `/org/noctalia/AppMenu/Active/menu` so QML can attach
//! to a constant address. Out of scope for v0.1.

use crate::{active::ActiveSnapshot, atspi, config::Config};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{watch, Mutex, Notify};
use tracing::{debug, info, warn};
use zbus::{interface, Connection};

/// Serialise the active snapshot to a JSON file. Truncating
/// in-place write — single open(O_WRONLY|O_TRUNC) + write + close,
/// preserving the file's inode across updates. Errors are logged
/// but non-fatal — the D-Bus proxy still publishes correctly.
///
/// **Why in-place, not tmpfile+rename (PR #41):** Quickshell's
/// `FileView.watchChanges` attaches an inotify watch to the inode,
/// not the path. The previous tmpfile+rename pattern replaced the
/// inode on every write — `IN_MOVE_SELF` fires once, then the new
/// inode is unwatched and silently misses subsequent writes. After
/// 17h+ of quickshell uptime across many bridge restarts the QML
/// widget would render empty until `systemctl --user restart
/// noctalia-shell`. In-place truncate keeps the inode stable;
/// inotify on the path resolves to the same node forever, and
/// `IN_MODIFY` fires reliably on every write.
///
/// **Tearing tradeoff:** the QML reader could in theory observe a
/// partial write between the truncate and the body write. Payload
/// is <4 KiB in practice (small menu trees) — Linux's `write(2)`
/// is atomic up to `PIPE_BUF` (4096) for regular files in single
/// syscalls, and `serde_json::to_string` produces one buffer
/// `std::fs::write` flushes in one syscall. Even when the payload
/// exceeds 4 KiB the QML reader simply hits a `JSON.parse` error
/// and skips that update — `BarWidget.qml` already handles this
/// with a try/catch (see ADR-0021).
///
/// `menu` carries the focused app's menubar tree as walked from
/// AT-SPI (v0.3 substrate). JSON shape is unchanged from v0.2's
/// dbusmenu walker so the QML widget needs zero edits — `service`
/// and `path` now point at AT-SPI accessibles instead of `DBusMenu`
/// items, but downstream click forwarding routes through the new
/// `atspi-click` subcommand which speaks the AT-SPI Action interface.
/// Schema version for `active.json`. Bumped when the field set or
/// semantic shape changes incompatibly. Plugin reads this and falls
/// back to its zero-paint slot when it doesn't recognise `v`.
///
/// History:
/// - v0.1: `{app_id, title}` (early DBusMenu placeholder)
/// - v0.2: `+menu_service +menu_path +menu (DBusMenu tree)`
/// - v0.3: `menu` carries the AT-SPI walked tree; `menu_service` and
///   `menu_path` retained for backward-compat but `menu` is the
///   authoritative source.
/// - v1 (post-spec-003): explicit `"v": 1` field for forward/backward
///   compat. Field set unchanged from v0.3.
/// - **v=1.1 (this commit, spec 005 FR-004)**: additive
///   `"source": "atspi" | "synthetic" | "empty"` top-level field —
///   `v` stays `1` because the change is wire-compat (consumers
///   ignoring the field continue to parse the rest of the payload).
///   See `specs/004-project-completion/contracts/active-json-schema.md`.
const ACTIVE_JSON_SCHEMA_VERSION: u32 = 1;

/// Provenance of the `menu` field in the active.json payload (spec
/// 005 FR-004). Serialises into the `source` top-level field; the
/// QML widget can render different placeholder styles based on the
/// source value (e.g. dim the bar when "synthetic", hide it when
/// "empty").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuSource {
    /// Menu walked from the focused app's AT-SPI accessible tree.
    Atspi,
    /// Menu synthesised from `app_id` because the focused app has no
    /// usable AT-SPI menubar. **Deprecated since v1.0.2** — the
    /// production proxy emits `Empty` instead per the honest-or-
    /// hidden UX (PR #47 / Pedro re-confirm 15/05/2026). Variant kept
    /// for API/serde stability and for future opt-in via per-widget
    /// setting; no live producer constructs it.
    #[allow(dead_code)]
    Synthetic,
    /// No focus on a window, OR focused app has no usable AT-SPI
    /// menubar (terminals, electron-no-a11y, native Wayland with no
    /// a11y plugin). `menu` is `null` in this case.
    Empty,
}

impl MenuSource {
    #[allow(dead_code)]
    fn as_str(self) -> &'static str {
        match self {
            Self::Atspi => "atspi",
            Self::Synthetic => "synthetic",
            Self::Empty => "empty",
        }
    }
}

/// Build + write the active-snapshot JSON, with **producer-side
/// dedup** — when the serialised payload is byte-identical to the
/// previous successful write, skip both the file write and the IPC
/// push entirely. Per Swarm H of the v3 best-practices synthesis,
/// this saves the round-trip cost (≈5 ms per `qs ipc call` spawn)
/// on the >80 % of bridge events that are no-op focus reshuffles
/// within the same app — the bridge currently re-pushes the full
/// menu tree on every `WindowFocusChanged`, but the payload only
/// changes when title or menu structure actually moves.
///
/// `last_body` is the caller-owned hash sentinel. Storing the full
/// body (≈4 KiB) is cheaper than hashing for a single-writer task
/// and lets us avoid the `xxhash`/`twox-hash` dep — comparison is
/// `as_deref() == Some(...)`, ≈100 ns on a memcmp fast-path.
fn write_active_json(
    path: &Path,
    snap: &ActiveSnapshot,
    menu: Option<&atspi::MenuItem>,
    source: MenuSource,
    last_body: &mut Option<String>,
) {
    let payload = serde_json::json!({
        "v": ACTIVE_JSON_SCHEMA_VERSION,
        "focus_pid": snap.focus_pid,
        "app_id": snap.app_id,
        "title": snap.title,
        "source": source.as_str(),
        "menu_service": snap.menu_service,
        "menu_path": snap.menu_path.as_ref().map_or("", |p| p.as_str()),
        "menu": menu,
    });
    let body = payload.to_string();

    if last_body.as_deref() == Some(body.as_str()) {
        // No-op publish: identical to last successful write. Skip
        // both file write and IPC push. Both sides idempotent so
        // skipping is safe; widget already has the latest state.
        tracing::trace!("active.json publish coalesced (payload unchanged)");
        return;
    }

    if let Err(e) = std::fs::write(path, &body) {
        warn!(error=?e, path=%path.display(), "active.json write failed");
        // Don't update last_body on write failure — next call should
        // retry the file write. IPC push still attempted because the
        // failure is on disk-write, not on the payload itself.
    } else {
        *last_body = Some(body.clone());
    }
    push_ipc_update(&body);
}

/// Push the latest snapshot JSON to the QML widget via Quickshell's
/// `qs ipc call` channel. The widget's `IpcHandler { target: "appmenu";
/// function update(json) {...} }` parses the body and updates state
/// directly — no inotify, no FileView debounce, no atomic-rename race
/// window. The active.json file remains as a cold-start fallback (the
/// widget's `FileView` reads it on first paint before the bridge has
/// pushed) and a debugging surface, but the steady-state path is push.
///
/// **Best-effort:** any spawn or call failure is logged at `debug` and
/// dropped. Quickshell may not be running at bridge start, the IPC
/// handler may not be registered yet, or `qs` may not be on PATH —
/// none of these are fatal because the file write above gives the
/// widget a recoverable surface either way.
///
/// **Why not async/native zbus?** Quickshell's IPC is implemented as
/// a CLI surface (`qs ipc call <target> <fn> <args>`), not a public
/// D-Bus interface. The `qs` binary is the canonical way to reach it
/// from out-of-process code. Spawning a short-lived child per focus
/// change is cheap (~5ms on this hardware) and avoids us linking
/// Quickshell internals.
fn push_ipc_update(body: &str) {
    use std::process::{Command, Stdio};
    let result = Command::new("qs")
        .args(["ipc", "call", "appmenu", "update", body])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .status();
    match result {
        Ok(s) if s.success() => {}
        Ok(s) => {
            // Non-fatal — Quickshell may not be running yet, or the
            // appmenu IpcHandler may not be loaded by the user's
            // shell config. Logged at debug so the steady-state log
            // stream stays quiet.
            tracing::debug!(status = ?s, "qs ipc call appmenu update returned non-zero");
        }
        Err(e) => {
            tracing::debug!(error = ?e, "qs ipc call spawn failed (qs not on PATH?)");
        }
    }
}

/// Plain-old-data view of the four properties exported on the active
/// proxy interface. Updated atomically as a unit (all four properties
/// always change together when focus moves), so a `Mutex` is the right
/// primitive — `RwLock` would be over-engineered for a single writer
/// and zero-to-occasional readers (D-Bus property reads from QML).
#[derive(Default, Debug, Clone)]
pub struct ActiveProxyState {
    /// D-Bus bus name of the focused application owning the menu.
    pub bus_name: String,
    /// D-Bus object path of the menu under `bus_name`.
    pub object_path: String,
    /// Wayland app-id of the focused window.
    pub app_id: String,
    /// Title of the focused window.
    pub title: String,
}

/// `org.noctalia.AppMenu.Active` D-Bus object. Wraps the state behind a
/// `Mutex` so async property accessors can read consistently while the
/// `run()` writer task pushes updates. The `refresh_kick` notifier is
/// the FR-007 wiring: the `atspi-click` CLI calls `RefreshActive` when
/// it detects a stale path, which wakes the run-loop and triggers an
/// immediate AT-SPI re-walk against the current snapshot.
#[derive(Clone, Default)]
pub struct ActiveProxy {
    inner: Arc<Mutex<ActiveProxyState>>,
    refresh_kick: Arc<Notify>,
}

impl ActiveProxy {
    /// Construct a fresh proxy with empty state. Call once per process.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a shared handle to the refresh-kick notifier. The
    /// `run()` task awaits on this alongside `active_rx.changed()`
    /// so a `RefreshActive` D-Bus call short-circuits the next focus
    /// event and re-walks immediately. Spec 005 FR-007.
    #[must_use]
    pub fn refresh_kick(&self) -> Arc<Notify> {
        self.refresh_kick.clone()
    }
}

#[interface(name = "org.noctalia.AppMenu.Active")]
impl ActiveProxy {
    #[zbus(property)]
    async fn bus_name(&self) -> String {
        self.inner.lock().await.bus_name.clone()
    }

    #[zbus(property)]
    async fn object_path(&self) -> String {
        self.inner.lock().await.object_path.clone()
    }

    #[zbus(property)]
    async fn app_id(&self) -> String {
        self.inner.lock().await.app_id.clone()
    }

    #[zbus(property)]
    async fn title(&self) -> String {
        self.inner.lock().await.title.clone()
    }

    /// FR-007 refresh trigger. The `atspi-click` CLI invokes this
    /// after `do_action` returns `MenuError::Stale` so the bridge
    /// re-walks the focused app's AT-SPI tree before the QML widget
    /// re-renders. Idempotent and best-effort: a notification is
    /// coalesced with any concurrent one (`tokio::Notify::notify_one`
    /// semantics), so a click storm cannot pile up walks.
    async fn refresh_active(&self) {
        tracing::debug!("RefreshActive D-Bus method received; kicking active loop");
        self.refresh_kick.notify_one();
    }
}

/// Long-running task: own `org.noctalia.AppMenu`, expose the active
/// proxy at `cfg.publish_path`, and keep its properties in sync with
/// the joiner's `ActiveSnapshot` stream.
///
/// Returns `Ok(())` when the watch channel closes (the joiner exited);
/// the caller's signal handler is the authoritative shutdown path.
pub async fn run(
    conn: Connection,
    client: atspi::AtspiClient,
    mut active_rx: watch::Receiver<ActiveSnapshot>,
    cfg: Config,
) -> anyhow::Result<()> {
    let proxy = ActiveProxy::new();
    let refresh_kick = proxy.refresh_kick();

    conn.object_server()
        .at(cfg.publish_path.as_str(), proxy.clone())
        .await?;

    conn.request_name(cfg.publish_service.as_str()).await?;
    info!(service = %cfg.publish_service, path = %cfg.publish_path, "owning active proxy");

    // File-IPC fallback: write the active snapshot to a JSON file
    // alongside the D-Bus proxy. Quickshell's `DBusObject` does not
    // exist as a public QML type (verified against v0.2.1 type list),
    // so the v0.1 plugin reads this file via `Quickshell.Io.FileView`
    // instead. ADR-0007 second-half (DBusMenu mirror) supersedes this
    // file-IPC path in v0.2.
    let cache_dir = std::env::var("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .ok()
        .or_else(|| {
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".cache"))
                .ok()
        })
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("noctalia-appmenu");
    let _ = std::fs::create_dir_all(&cache_dir);
    let active_json_path = cache_dir.join("active.json");

    // Emit an initial empty snapshot so the file exists at startup
    // (matters for first-load of the QML widget).
    // Caller-owned dedup state for write_active_json. Persists across
    // loop iterations so consecutive identical payloads coalesce.
    let mut last_body: Option<String> = None;

    write_active_json(
        &active_json_path,
        &ActiveSnapshot::empty(),
        None,
        MenuSource::Empty,
        &mut last_body,
    );

    loop {
        // FR-007 wake-up: in addition to focus-driven changes from
        // `active_rx`, the loop wakes on a `RefreshActive` D-Bus
        // call so a stale-path click triggers an immediate re-walk
        // against the current snapshot. On the notify branch we
        // intentionally skip `borrow_and_update` of a fresh value —
        // we want to re-walk the same focused app, not consume a
        // pending focus event the run-loop hasn't seen yet.
        tokio::select! {
            r = active_rx.changed() => {
                if r.is_err() {
                    break;
                }
            }
            () = refresh_kick.notified() => {
                debug!("RefreshActive kick observed; re-walking current snapshot");
                // Suppress producer-side dedup so the re-walk
                // always emits, even if the eventual menu tree
                // happens to serialise byte-identical to the
                // pre-stale tree — the click handler is waiting
                // on a real update, not a coalesced no-op.
                last_body = None;
            }
        }
        let snapshot = active_rx.borrow_and_update().clone();

        // Eager publish: write app_id + title to active.json with
        // menu:null so the bar updates instantly, then refine the
        // menu field once the AT-SPI walk completes. Without this
        // the QML widget waits up to ~3.6s (timeout + retries)
        // before showing the new app's title — a regression vs the
        // pre-retry v0.3.0-alpha.6 behaviour (codex review of #40).
        // The eager-publish `source` is `empty` when there is no
        // focus, `atspi` when we expect the upcoming walk to succeed;
        // the trailing write at the bottom of the loop overrides
        // with the real provenance.
        let eager_source = if snapshot.focus_pid == 0 {
            MenuSource::Empty
        } else {
            MenuSource::Atspi
        };
        write_active_json(
            &active_json_path,
            &snapshot,
            None,
            eager_source,
            &mut last_body,
        );
        publish_props(&conn, &cfg, &proxy, &snapshot).await?;

        // v0.3 substrate: walk the focused app's AT-SPI menubar
        // with up-to-3 cancellable retries. Pass-1 PID resolution
        // is sequential and can blow the per-call timeout when a
        // misbehaving registered app is slow; on the first walk
        // after bridge restart the registry can also be cold.
        // Retry-on-None gives the registry a chance to warm. The
        // sleep is cancellation-aware so a user alt-tabbing during
        // backoff doesn't get stuck rendering the old menu.
        // Apps without an AT-SPI menu (terminals, electron-no-a11y)
        // pay 200+400=600ms extra per focus before settling on
        // null — accepted as the cost of universal correctness.
        // v1.0.6 — fast paths BEFORE the AT-SPI walk:
        //
        // 1. Skip-list: known no-menubar apps (terminals, X11 hosts,
        //    Firefox/Chromium per Pedro field report) return None
        //    immediately — no D-Bus traffic at all. ~0ms.
        //
        // 2. Cache hit: same (app_id, pid) walked in the last
        //    MENU_CACHE_TTL (30s) returns the cached value (positive
        //    or negative) instantly — also ~0ms. Re-focusing an app
        //    Pedro just had focused is the common case.
        //
        // 3. Cache miss + not in skip-list: walk AT-SPI ONCE (no
        //    retry loop), populate cache, move on. The retry-on-None
        //    loop the v0.3 code shipped paid 200+400=600ms backoff
        //    on every cache miss for an app without a menubar — the
        //    cache eliminates that for steady-state.
        let menu: Option<atspi::MenuItem> = if snapshot.focus_pid == 0 {
            None
        } else if atspi::is_known_no_menubar(&snapshot.app_id) {
            debug!(
                app_id = %snapshot.app_id,
                pid = snapshot.focus_pid,
                "skip-list: known no-menubar app, no AT-SPI walk"
            );
            None
        } else if let Some(cached) =
            atspi::cached_menu_for_pid(&snapshot.app_id, snapshot.focus_pid)
        {
            debug!(
                app_id = %snapshot.app_id,
                pid = snapshot.focus_pid,
                cached_some = cached.is_some(),
                "cache hit; skipping AT-SPI walk"
            );
            cached
        } else {
            match atspi::fetch_menubar_for_pid(&client, snapshot.focus_pid, Some(&snapshot.app_id))
                .await
            {
                Ok(opt) => {
                    debug!(
                        pid = snapshot.focus_pid,
                        top_level = opt.as_ref().map(|m| m.children.len()).unwrap_or(0),
                        cached_negative = opt.is_none(),
                        "walked atspi menubar; caching"
                    );
                    atspi::cache_menu_for_pid(&snapshot.app_id, snapshot.focus_pid, opt.clone());
                    opt
                }
                Err(e) => {
                    warn!(
                        error = ?e,
                        pid = snapshot.focus_pid,
                        "atspi walk failed; widget falls back to placeholder"
                    );
                    None
                }
            }
        };

        // Spec 011 — honest-or-hidden UX (Pedro's PR #47, 15/05/2026
        // re-confirm): when the focused app has no usable AT-SPI
        // menubar (terminals, electron-no-a11y, native Wayland with
        // no a11y plugin), DROP the synthetic pseudo-menu and emit
        // `menu: null` so the bar widget collapses to its zero-paint
        // stable slot. macOS has 100% coverage because Apple owns
        // Cocoa; Wayland-niri can't, so we don't pretend.
        //
        // The synthetic_menu function is preserved in atspi.rs for
        // the test surface and for any future opt-in via per-widget
        // setting; the production proxy emits None.
        let (final_menu, final_source): (Option<atspi::MenuItem>, MenuSource) =
            if snapshot.focus_pid == 0 {
                (None, MenuSource::Empty)
            } else {
                match menu {
                    Some(m) => (Some(m), MenuSource::Atspi),
                    None => (None, MenuSource::Empty),
                }
            };

        write_active_json(
            &active_json_path,
            &snapshot,
            final_menu.as_ref(),
            final_source,
            &mut last_body,
        );
    }

    Ok(())
}

/// Update the in-process `ActiveProxy` state with `snapshot` and
/// emit D-Bus property-change signals on the active proxy interface.
/// Called on every focus change so the QML widget's DBus binding
/// wakes up.
async fn publish_props(
    conn: &Connection,
    cfg: &Config,
    proxy: &ActiveProxy,
    snapshot: &ActiveSnapshot,
) -> anyhow::Result<()> {
    {
        let mut s = proxy.inner.lock().await;
        s.bus_name = snapshot.menu_service.clone();
        s.object_path = snapshot
            .menu_path
            .as_ref()
            .map(|p| p.as_str().to_string())
            .unwrap_or_default();
        s.app_id = snapshot.app_id.clone();
        s.title = snapshot.title.clone();
    }
    let iface_ref = conn
        .object_server()
        .interface::<_, ActiveProxy>(cfg.publish_path.as_str())
        .await?;
    let iface = iface_ref.get().await;
    iface.bus_name_changed(iface_ref.signal_context()).await?;
    iface
        .object_path_changed(iface_ref.signal_context())
        .await?;
    iface.app_id_changed(iface_ref.signal_context()).await?;
    iface.title_changed(iface_ref.signal_context()).await?;
    Ok(())
}
