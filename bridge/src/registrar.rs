//! com.canonical.AppMenu.Registrar consumer.
//!
//! We listen for RegisterWindow / UnregisterWindow signals, discard the
//! `windowId` argument (X11-only, ADR-0004), and resolve the registering
//! connection's PID via org.freedesktop.DBus.GetConnectionUnixProcessID.
//!
//! Output: a `pid -> (busName, objectPath)` map maintained as a
//! `watch::Sender<MenuMap>`.

use crate::config::Config;
use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::sync::watch;
use tracing::{debug, info, warn};
use zbus::{
    fdo::DBusProxy,
    proxy,
    zvariant::{ObjectPath, OwnedObjectPath},
    Connection,
};

#[derive(Debug, Clone, Default)]
pub struct MenuMap {
    pub by_pid: HashMap<u32, (String, OwnedObjectPath)>,
}

#[proxy(
    interface = "com.canonical.AppMenu.Registrar",
    default_service = "com.canonical.AppMenu.Registrar",
    default_path = "/com/canonical/AppMenu/Registrar"
)]
trait Registrar {
    /// Args: window_id (u32, X11 XID — IGNORED), menu_object_path.
    /// Sender of this signal IS the app whose menu was registered;
    /// we resolve its PID via DBus.GetConnectionUnixProcessID.
    #[zbus(signal)]
    fn window_registered(&self, window_id: u32, service_name: String, menu_path: ObjectPath<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    fn window_unregistered(&self, window_id: u32) -> zbus::Result<()>;

    fn get_menu_for_window(&self, window_id: u32) -> zbus::Result<(String, OwnedObjectPath)>;
}

pub async fn run(conn: Connection, tx: watch::Sender<MenuMap>, _cfg: Config) -> Result<()> {
    let registrar = RegistrarProxy::new(&conn).await?;
    let dbus = DBusProxy::new(&conn).await?;

    let mut map = MenuMap::default();

    let mut registered = registrar
        .receive_window_registered()
        .await
        .context("subscribing to WindowRegistered")?;
    let mut unregistered = registrar
        .receive_window_unregistered()
        .await
        .context("subscribing to WindowUnregistered")?;

    // Many registrars do not enumerate-on-connect, so we rely entirely
    // on the live signal stream from this point on. Late-bound apps
    // re-register when they reconnect.
    info!("registrar consumer ready");

    loop {
        tokio::select! {
            Some(sig) = futures::StreamExt::next(&mut registered) => {
                match sig.args() {
                    Ok(args) => {
                        // The signal's *sender* on the bus is the app process.
                        // The argument `service_name` is the name *the registrar
                        // suggests* (often == sender). We use the sender — that's
                        // the connection whose PID we're authorised to resolve.
                        if let Some(sender) = sig.message().header().sender().map(|s| s.to_string()) {
                            let bus_name = sender
                                .as_str()
                                .try_into()
                                .with_context(|| format!("parsing bus name '{sender}'"))?;
                            match dbus.get_connection_unix_process_id(bus_name).await {
                                Ok(pid) => {
                                    let bus = args.service_name.clone();
                                    let path: OwnedObjectPath = args.menu_path.clone().into();
                                    debug!(pid, bus=%bus, path=%path.as_str(), "menu registered");
                                    map.by_pid.insert(pid, (bus, path));
                                    let _ = tx.send(map.clone());
                                }
                                Err(e) => warn!(?e, "GetConnectionUnixProcessID failed"),
                            }
                        }
                    }
                    Err(e) => warn!(?e, "could not parse WindowRegistered args"),
                }
            }
            Some(sig) = futures::StreamExt::next(&mut unregistered) => {
                // We don't trust the X11 windowId here either. Best-effort:
                // remove any entry whose sender PID matches the signal
                // sender. Apps that crash will be cleaned up by a
                // periodic stale-PID sweep (planned in spec 002 — bridge mirror). // nosemgrep
                if let Some(sender) = sig.message().header().sender().map(|s| s.to_string()) {
                    let bus_name = sender
                        .as_str()
                        .try_into()
                        .with_context(|| format!("parsing bus name '{sender}'"))?;
                    if let Ok(pid) = dbus.get_connection_unix_process_id(bus_name).await {
                        if map.by_pid.remove(&pid).is_some() {
                            let _ = tx.send(map.clone());
                        }
                    }
                }
            }
        }
    }
}
