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
use tokio::sync::{watch, Mutex};
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
fn write_active_json(path: &Path, snap: &ActiveSnapshot, menu: Option<&atspi::MenuItem>) {
    let payload = serde_json::json!({
        "focus_pid": snap.focus_pid,
        "app_id": snap.app_id,
        "title": snap.title,
        "menu_service": snap.menu_service,
        "menu_path": snap.menu_path.as_ref().map_or("", |p| p.as_str()),
        "menu": menu,
    });
    if let Err(e) = std::fs::write(path, payload.to_string()) {
        warn!(error=?e, path=%path.display(), "active.json write failed");
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
/// `run()` writer task pushes updates.
#[derive(Clone, Default)]
pub struct ActiveProxy {
    inner: Arc<Mutex<ActiveProxyState>>,
}

impl ActiveProxy {
    /// Construct a fresh proxy with empty state. Call once per process.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
}

/// Long-running task: own `org.noctalia.AppMenu`, expose the active
/// proxy at `cfg.publish_path`, and keep its properties in sync with
/// the joiner's `ActiveSnapshot` stream.
///
/// Returns `Ok(())` when the watch channel closes (the joiner exited);
/// the caller's signal handler is the authoritative shutdown path.
pub async fn run(
    conn: Connection,
    mut active_rx: watch::Receiver<ActiveSnapshot>,
    cfg: Config,
) -> anyhow::Result<()> {
    let proxy = ActiveProxy::new();

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
    write_active_json(&active_json_path, &ActiveSnapshot::empty(), None);

    loop {
        if active_rx.changed().await.is_err() {
            break;
        }
        let mut snapshot = active_rx.borrow_and_update().clone();

        // Eager publish: write app_id + title to active.json with
        // menu:null so the bar updates instantly, then refine the
        // menu field once the AT-SPI walk completes. Without this
        // the QML widget waits up to ~3.6s (timeout + retries)
        // before showing the new app's title — a regression vs the
        // pre-retry v0.3.0-alpha.6 behaviour (codex review of #40).
        write_active_json(&active_json_path, &snapshot, None);
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
        let menu: Option<atspi::MenuItem> = if snapshot.focus_pid != 0 {
            let mut found: Option<atspi::MenuItem> = None;
            let mut attempt: u32 = 0;
            loop {
                match atspi::fetch_menubar_for_pid(snapshot.focus_pid, Some(&snapshot.app_id)).await
                {
                    Ok(Some(m)) => {
                        debug!(
                            pid = snapshot.focus_pid,
                            top_level = m.children.len(),
                            attempt,
                            "walked atspi menubar"
                        );
                        found = Some(m);
                        break;
                    }
                    Ok(None) if attempt < 2 => {
                        let backoff = std::time::Duration::from_millis(200 * (1u64 << attempt));
                        tokio::select! {
                            () = tokio::time::sleep(backoff) => {
                                attempt += 1;
                            }
                            r = active_rx.changed() => {
                                if r.is_err() {
                                    break;
                                }
                                snapshot = active_rx.borrow_and_update().clone();
                                write_active_json(&active_json_path, &snapshot, None);
                                publish_props(&conn, &cfg, &proxy, &snapshot).await?;
                                if snapshot.focus_pid == 0 {
                                    break;
                                }
                                attempt = 0;
                            }
                        }
                    }
                    Ok(None) => {
                        debug!(pid = snapshot.focus_pid, "no atspi menubar for focused app");
                        break;
                    }
                    Err(e) => {
                        warn!(
                            error = ?e,
                            pid = snapshot.focus_pid,
                            "atspi walk failed; widget falls back to placeholder"
                        );
                        break;
                    }
                }
            }
            found
        } else {
            None
        };

        // Synthetic fallback (PR #42): when AT-SPI returns no menu
        // (terminals, electron-no-a11y, native Wayland with no a11y
        // plugin), surface a universal "Window" submenu wired to
        // niri-IPC actions so the bar always shows something useful.
        // macOS philosophy: every focused app has menus, even when
        // the app itself doesn't ship them.
        let final_menu: Option<atspi::MenuItem> = match menu {
            Some(m) => Some(m),
            None if snapshot.focus_pid != 0 => Some(atspi::synthetic_window_menu(&snapshot.app_id)),
            None => None,
        };
        if final_menu.is_some() {
            write_active_json(&active_json_path, &snapshot, final_menu.as_ref());
        }
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
