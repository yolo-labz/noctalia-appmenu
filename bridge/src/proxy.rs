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
//! Future work (ADR-0007 second-half): mirror the upstream DBusMenu
//! itself under `/org/noctalia/AppMenu/Active/menu` so QML can attach
//! to a constant address. Out of scope for v0.1.

use crate::{active::ActiveSnapshot, config::Config};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use tracing::{info, warn};
use zbus::{interface, Connection};

/// Serialise the active snapshot to a JSON file. Atomic write:
/// write to `<path>.tmp` then rename, so file-watching consumers
/// never observe a partially-written file. Errors are logged but
/// non-fatal — the D-Bus proxy still publishes correctly.
fn write_active_json(path: &Path, snap: &ActiveSnapshot) {
    let payload = serde_json::json!({
        "focus_pid": snap.focus_pid,
        "app_id": snap.app_id,
        "title": snap.title,
        "menu_service": snap.menu_service,
        "menu_path": snap.menu_path.as_ref().map(|p| p.as_str()).unwrap_or(""),
    });
    let tmp = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp, payload.to_string()) {
        warn!(error=?e, path=%path.display(), "active.json write failed");
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        warn!(error=?e, path=%path.display(), "active.json rename failed");
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
    write_active_json(&active_json_path, &ActiveSnapshot::empty());

    while active_rx.changed().await.is_ok() {
        let snapshot = active_rx.borrow_and_update().clone();
        write_active_json(&active_json_path, &snapshot);
        {
            let mut s = proxy.inner.lock().await;
            s.bus_name = snapshot.menu_service;
            s.object_path = snapshot
                .menu_path
                .map(|p| p.as_str().to_string())
                .unwrap_or_default();
            s.app_id = snapshot.app_id;
            s.title = snapshot.title;
        }

        // Notify property changes so QML's DBus binding wakes up.
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
    }

    Ok(())
}
