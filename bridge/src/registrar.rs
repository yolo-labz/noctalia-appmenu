//! `com.canonical.AppMenu.Registrar` SERVER-SIDE implementation.
//!
//! v0.1 was a CLIENT of an upstream registrar (typically
//! `vala-panel-appmenu-daemon`). On Pedro's NixOS desktop no such
//! daemon ships — `vala-panel-appmenu` is not packaged in nixpkgs (PR
//! #374302 closed unmerged). Without a registrar service owning
//! `com.canonical.AppMenu.Registrar` on the session bus, every
//! Qt/GTK app that tries to call `RegisterWindow` gets a
//! `ServiceUnknown` error and silently gives up — no menus ever
//! reach our bridge.
//!
//! v0.2 flips this: the bridge BECOMES the registrar. It owns the
//! well-known name, implements `RegisterWindow` / `UnregisterWindow`
//! / `GetMenuForWindow` per the canonical spec, and tracks the
//! `xid → (busName, menuPath)` mapping internally. Apps register
//! against us; we route their menu paths to the focus subsystem
//! (downstream consumers see a `pid → (busName, menuPath)` map
//! exactly as before, so `active.rs` is untouched).
//!
//! ADR-0001 said "reconsidered in v0.2 if vala-panel-appmenu becomes
//! unmaintained" — that condition is now load-bearing.
//! ADR-0022 documents the v0.2 server-side decision.
//!
//! ## Wayland xid mapping
//!
//! `DBusMenu`'s `RegisterWindow(xid, path)` was designed for X11 where
//! every window has a unique XID. On Wayland (including Xwayland),
//! the xid is either:
//!   - A real X11 xid for Xwayland-bridged apps (Anki, gimp, etc.)
//!   - A synthetic id (Qt6 hashes the `wl_surface`)
//!   - Zero (apps that don't bother)
//!
//! We treat the xid opaquely as a key — never look up the X server
//! to validate it. The pid lookup via D-Bus.GetConnectionUnixProcessID
//! is the authoritative cross-reference for `active.rs`.

use crate::config::Config;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use tracing::{debug, info};
use zbus::{
    fdo::DBusProxy,
    interface,
    message::Header,
    object_server::SignalContext,
    zvariant::{ObjectPath, OwnedObjectPath},
    Connection,
};

/// Public output: `pid → (busName, menuPath)` map. `active.rs`
/// consumes this via `watch::Receiver`. Backward-compatible with
/// v0.1's CLIENT implementation.
#[derive(Debug, Clone, Default)]
pub struct MenuMap {
    pub by_pid: HashMap<u32, (String, OwnedObjectPath)>,
}

/// Internal state: the canonical xid → registration table that the
/// registrar interface exposes. Plus a parallel xid → pid index so
/// `UnregisterWindow(xid)` can keep `by_pid` consistent without an
/// O(n) scan of the menu map.
#[derive(Default)]
struct RegistrarState {
    /// Canonical D-Bus-spec map: xid → (busName, menuPath).
    by_xid: HashMap<u32, (String, OwnedObjectPath)>,
    /// Parallel: xid → resolved pid, populated at register time so
    /// unregister can clean up `MenuMap.by_pid` cheaply even after
    /// the registering connection has gone away (and its PID can no
    /// longer be looked up via DBus.GetConnectionUnixProcessID).
    pid_by_xid: HashMap<u32, u32>,
}

/// `com.canonical.AppMenu.Registrar` server. Holds shared state
/// behind a `Mutex` so the interface methods (which run on the zbus
/// dispatch thread) and the `tx` watch publisher can both update it
/// safely.
pub struct AppMenuRegistrar {
    state: Arc<Mutex<RegistrarState>>,
    tx: watch::Sender<MenuMap>,
    /// Used to resolve a registering connection's PID. Stored once at
    /// startup so each `RegisterWindow` call doesn't re-build the
    /// proxy.
    dbus: DBusProxy<'static>,
}

#[interface(name = "com.canonical.AppMenu.Registrar")]
impl AppMenuRegistrar {
    /// Apps call this to publish their menu. The `xid` is the X11
    /// window id (or a Wayland synthetic). `menu_path` is the object
    /// path of their `com.canonical.dbusmenu` service exported on
    /// the calling connection.
    ///
    /// `#[zbus(signal_context)]` injects the `SignalContext` so we
    /// can fire `WindowRegistered` after updating state.
    /// `#[zbus(header)]` exposes the message header so we can extract
    /// the calling connection's well-known sender name.
    async fn register_window(
        &self,
        #[zbus(signal_context)] emitter: SignalContext<'_>,
        #[zbus(header)] hdr: Header<'_>,
        xid: u32,
        menu_path: ObjectPath<'_>,
    ) -> zbus::fdo::Result<()> {
        let sender = hdr
            .sender()
            .ok_or_else(|| zbus::fdo::Error::Failed("no sender on RegisterWindow".into()))?
            .to_string();
        let bus_name_for_proxy = sender
            .as_str()
            .try_into()
            .map_err(|e| zbus::fdo::Error::Failed(format!("bad bus name {sender}: {e}")))?;
        let pid = self
            .dbus
            .get_connection_unix_process_id(bus_name_for_proxy)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(format!("pid lookup failed: {e}")))?;

        let bus = sender;
        let owned_path: OwnedObjectPath = menu_path.clone().into();

        {
            let mut s = self.state.lock().await;
            s.by_xid.insert(xid, (bus.clone(), owned_path.clone()));
            s.pid_by_xid.insert(xid, pid);
        }

        debug!(xid, pid, bus=%bus, path=%owned_path.as_str(), "RegisterWindow");
        self.publish().await;

        // Echo the standard signal so other consumers (KDE Plasma's
        // appmenu applet, future Quickshell native consumer) can
        // subscribe normally.
        let menu_path_ref = menu_path.as_ref();
        let _ =
            AppMenuRegistrar::window_registered(&emitter, xid, bus.as_str(), &menu_path_ref).await;

        Ok(())
    }

    /// Apps call this on exit. We trust the xid because we keyed on
    /// it at register time — no PID re-lookup needed (and indeed the
    /// connection may already be gone).
    async fn unregister_window(
        &self,
        #[zbus(signal_context)] emitter: SignalContext<'_>,
        xid: u32,
    ) -> zbus::fdo::Result<()> {
        let removed_pid = {
            let mut s = self.state.lock().await;
            s.by_xid.remove(&xid);
            s.pid_by_xid.remove(&xid)
        };
        debug!(xid, pid=?removed_pid, "UnregisterWindow");
        self.publish().await;

        let _ = AppMenuRegistrar::window_unregistered(&emitter, xid).await;
        Ok(())
    }

    /// Bar widgets call this on focus change to find the menu for a
    /// given xid. Returns the registered (busName, menuPath) pair.
    /// Errors with `NotFound` if the xid is not registered — apps
    /// that haven't called `RegisterWindow` won't appear here.
    async fn get_menu_for_window(&self, xid: u32) -> zbus::fdo::Result<(String, OwnedObjectPath)> {
        let s = self.state.lock().await;
        s.by_xid
            .get(&xid)
            .cloned()
            .ok_or_else(|| zbus::fdo::Error::Failed(format!("no menu registered for xid {xid}")))
    }

    /// Standard signals — emitted in addition to method-call return.
    /// Defining them here makes the interface introspectable and lets
    /// downstream tools (busctl tree, kdialog, plasma's appmenu
    /// applet) discover us.
    #[zbus(signal)]
    async fn window_registered(
        emitter: &SignalContext<'_>,
        xid: u32,
        service_name: &str,
        menu_path: &ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn window_unregistered(emitter: &SignalContext<'_>, xid: u32) -> zbus::Result<()>;
}

impl AppMenuRegistrar {
    async fn publish(&self) {
        let s = self.state.lock().await;
        let mut by_pid = HashMap::new();
        for (xid, (bus, path)) in &s.by_xid {
            if let Some(pid) = s.pid_by_xid.get(xid) {
                by_pid.insert(*pid, (bus.clone(), path.clone()));
            }
        }
        let _ = self.tx.send(MenuMap { by_pid });
    }
}

/// Long-running task: register the well-known name on the session
/// bus and serve the registrar interface.
///
/// **Name-collision behaviour:** if `vala-panel-appmenu-daemon` (or
/// any other registrar) already owns the name, this `request_name`
/// call fails. We log a warning and continue with an empty
/// `MenuMap` — `active.rs` then sees no registered menus, the bar
/// widget shows the v0.1 fallback. The user can investigate which
/// process is hogging the name (`busctl --user list`) and disable
/// it.
///
/// **Future hardening:** add `RequestNameFlags::ALLOW_REPLACEMENT |
/// REPLACE_EXISTING` to take over from a stale daemon. Risk:
/// stomping on a legitimate session-wide registrar (e.g., user has
/// KDE Plasma running as their actual desktop and noctalia as a
/// supplemental bar). Default to safe-failure for v0.2.0.
pub async fn run(conn: Connection, tx: watch::Sender<MenuMap>, _cfg: Config) -> Result<()> {
    let dbus = DBusProxy::new(&conn)
        .await
        .context("connecting to org.freedesktop.DBus")?;

    let registrar = AppMenuRegistrar {
        state: Arc::new(Mutex::new(RegistrarState::default())),
        tx,
        dbus,
    };

    conn.object_server()
        .at("/com/canonical/AppMenu/Registrar", registrar)
        .await
        .context("exporting registrar at /com/canonical/AppMenu/Registrar")?;

    conn.request_name("com.canonical.AppMenu.Registrar")
        .await
        .context(
            "owning com.canonical.AppMenu.Registrar — another \
             registrar daemon (vala-panel-appmenu-daemon, etc.) may \
             already hold the name",
        )?;

    info!("registrar server ready (v0.2.0-alpha)");

    // Park the task forever — the interface lives on the connection's
    // object server and runs from zbus's dispatcher. We just need to
    // not return.
    std::future::pending::<()>().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_map_default_is_empty() {
        let m = MenuMap::default();
        assert!(m.by_pid.is_empty());
    }

    #[test]
    fn registrar_state_default_is_empty() {
        let s = RegistrarState::default();
        assert!(s.by_xid.is_empty());
        assert!(s.pid_by_xid.is_empty());
    }

    #[test]
    fn registrar_state_insert_and_remove() {
        let mut s = RegistrarState::default();
        let path: OwnedObjectPath = ObjectPath::from_str_unchecked("/foo/bar").into();
        s.by_xid.insert(123, (":1.42".to_string(), path.clone()));
        s.pid_by_xid.insert(123, 1234);

        assert_eq!(s.by_xid.len(), 1);
        assert_eq!(s.pid_by_xid.get(&123), Some(&1234));

        s.by_xid.remove(&123);
        s.pid_by_xid.remove(&123);
        assert!(s.by_xid.is_empty());
        assert!(s.pid_by_xid.is_empty());
    }
}
