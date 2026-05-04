//! Joins the niri focus stream and the registrar menu map into a single
//! debounced "active app menu" snapshot.

use crate::{config::Config, niri::FocusEvent, registrar::MenuMap};
use std::time::Duration;
use tokio::sync::watch;
use tracing::debug;
use zbus::zvariant::OwnedObjectPath;

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveSnapshot {
    pub focus_pid: u32,
    pub app_id: String,
    pub title: String,
    pub menu_service: String,
    pub menu_path: Option<OwnedObjectPath>,
}

impl ActiveSnapshot {
    pub fn empty() -> Self {
        Self {
            focus_pid: 0,
            app_id: String::new(),
            title: String::new(),
            menu_service: String::new(),
            menu_path: None,
        }
    }
}

pub async fn run(
    mut focus_rx: watch::Receiver<Option<FocusEvent>>,
    mut menus_rx: watch::Receiver<MenuMap>,
    tx: watch::Sender<ActiveSnapshot>,
    cfg: Config,
) -> anyhow::Result<()> {
    let debounce = Duration::from_millis(cfg.focus_debounce_ms);

    loop {
        // Trail-edge debounce on either input.
        tokio::select! {
            _ = focus_rx.changed() => {}
            _ = menus_rx.changed() => {}
        }

        // Wait for stillness before publishing.
        tokio::time::sleep(debounce).await;

        let focus = focus_rx.borrow().clone();
        let menus = menus_rx.borrow().clone();

        let snapshot = match focus {
            None => ActiveSnapshot::empty(),
            Some(f) => {
                let menu = menus.by_pid.get(&f.pid).cloned();
                ActiveSnapshot {
                    focus_pid: f.pid,
                    app_id: f.app_id,
                    title: f.title,
                    menu_service: menu.as_ref().map(|(s, _)| s.clone()).unwrap_or_default(),
                    menu_path: menu.map(|(_, p)| p),
                }
            }
        };

        debug!(?snapshot, "active snapshot");
        let _ = tx.send(snapshot);
    }
}
