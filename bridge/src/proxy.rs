//! Owns `org.noctalia.AppMenu` on the session bus and exposes
//! /org/noctalia/AppMenu/Active with properties:
//!
//!   - busName     : string
//!   - objectPath  : string
//!   - appId       : string
//!   - title       : string
//!
//! The QML widget binds to these. Currently the QML widget then
//! independently attaches a Quickshell DBusMenuHandle to (busName,
//! objectPath).
//!
//! Future work (ADR-0007 second-half): mirror the upstream DBusMenu
//! itself under /org/noctalia/AppMenu/Active so QML can attach to a
//! constant address. Out of scope for v0.1.

use crate::{active::ActiveSnapshot, config::Config};
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tracing::info;
use zbus::{interface, Connection};

pub struct ActiveProxyState {
    pub bus_name: String,
    pub object_path: String,
    pub app_id: String,
    pub title: String,
}

impl Default for ActiveProxyState {
    fn default() -> Self {
        Self {
            bus_name: String::new(),
            object_path: String::new(),
            app_id: String::new(),
            title: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct ActiveProxy {
    inner: Arc<RwLock<ActiveProxyState>>,
}

impl ActiveProxy {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ActiveProxyState::default())),
        }
    }
}

#[interface(name = "org.noctalia.AppMenu.Active")]
impl ActiveProxy {
    #[zbus(property)]
    async fn bus_name(&self) -> String {
        self.inner.read().await.bus_name.clone()
    }

    #[zbus(property)]
    async fn object_path(&self) -> String {
        self.inner.read().await.object_path.clone()
    }

    #[zbus(property)]
    async fn app_id(&self) -> String {
        self.inner.read().await.app_id.clone()
    }

    #[zbus(property)]
    async fn title(&self) -> String {
        self.inner.read().await.title.clone()
    }
}

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

    while active_rx.changed().await.is_ok() {
        let snapshot = active_rx.borrow().clone();
        {
            let mut s = proxy.inner.write().await;
            s.bus_name = snapshot.menu_service;
            s.object_path = snapshot
                .menu_path
                .map(|p| p.as_str().to_string())
                .unwrap_or_default();
            s.app_id = snapshot.app_id;
            s.title = snapshot.title;
        }

        // Notify property changes
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
