//! Joins the niri focus stream and the registrar menu map into a
//! single debounced "active app menu" snapshot.
//!
//! The joiner is a stateless reducer: take the current `Option<FocusEvent>`
//! from niri and the current `MenuMap` from the registrar consumer,
//! return the cross-product as an `ActiveSnapshot`. Debounce sits on the
//! input side — see [`run`] for the loop.

use crate::{config::Config, niri::FocusEvent, registrar::MenuMap};
use std::time::Duration;
use tokio::sync::watch;
use tracing::debug;
use zbus::zvariant::OwnedObjectPath;

/// Joined state published to the proxy task. All four downstream D-Bus
/// properties derive from this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSnapshot {
    /// PID of the currently-focused window's owning client.
    pub focus_pid: u32,
    /// Wayland app-id of the focused window.
    pub app_id: String,
    /// Title of the focused window.
    pub title: String,
    /// Bus name of the focused app's registered menu (empty when none).
    pub menu_service: String,
    /// Object path of the focused app's menu (`None` when no menu).
    pub menu_path: Option<OwnedObjectPath>,
}

impl ActiveSnapshot {
    /// The "no focus" / "no menu" baseline, used at startup and on
    /// graceful drop-focus events from niri.
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

/// Pure reducer — produced an `ActiveSnapshot` from the current state of
/// the two upstream watch channels. Extracted from `run()` so it can be
/// unit-tested independently of the debounce loop.
pub fn snapshot(focus: Option<&FocusEvent>, menus: &MenuMap) -> ActiveSnapshot {
    match focus {
        None => ActiveSnapshot::empty(),
        Some(f) => {
            let menu = menus.by_pid.get(&f.pid).cloned();
            ActiveSnapshot {
                focus_pid: f.pid,
                app_id: f.app_id.clone(),
                title: f.title.clone(),
                menu_service: menu.as_ref().map(|(s, _)| s.clone()).unwrap_or_default(),
                menu_path: menu.map(|(_, p)| p),
            }
        }
    }
}

/// Long-running task: trail-edge-debounce changes on either input
/// channel and publish a fresh `ActiveSnapshot` to `tx`.
///
/// `tokio::select!` semantics: the loop body fires when *either* input
/// has changed. We then sleep for `cfg.focus_debounce_ms` and read the
/// latest values via `borrow_and_update()`, which acks both watch
/// channels for the next iteration. If a third change arrives during
/// the sleep, the next loop iteration consumes it via the next
/// `select!`. There is no requirement for "both ready"; either-changed
/// is the contract — see ADR-0009.
///
/// Returns only on watch channel close (sender dropped) or error.
pub async fn run(
    mut focus_rx: watch::Receiver<Option<FocusEvent>>,
    mut menus_rx: watch::Receiver<MenuMap>,
    tx: watch::Sender<ActiveSnapshot>,
    cfg: Config,
) -> anyhow::Result<()> {
    let debounce = Duration::from_millis(cfg.focus_debounce_ms);

    loop {
        tokio::select! {
            _ = focus_rx.changed() => {}
            _ = menus_rx.changed() => {}
        }

        // Wait for stillness before publishing.
        tokio::time::sleep(debounce).await;

        let focus = focus_rx.borrow_and_update().clone();
        let menus = menus_rx.borrow_and_update().clone();
        let snap = snapshot(focus.as_ref(), &menus);

        debug!(?snap, "active snapshot");
        let _ = tx.send(snap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use zbus::zvariant::ObjectPath;

    fn focus(pid: u32, app: &str) -> FocusEvent {
        FocusEvent {
            winid: 1,
            pid,
            app_id: app.into(),
            title: "t".into(),
        }
    }

    fn op(s: &str) -> OwnedObjectPath {
        ObjectPath::try_from(s).unwrap().into()
    }

    #[test]
    fn no_focus_yields_empty() {
        let menus = MenuMap::default();
        assert_eq!(snapshot(None, &menus), ActiveSnapshot::empty());
    }

    #[test]
    fn focus_with_matching_menu_populates_all_fields() {
        let mut by_pid = HashMap::new();
        by_pid.insert(
            123u32,
            ("org.example.App".into(), op("/org/example/App/menu")),
        );
        let menus = MenuMap { by_pid };
        let f = focus(123, "App");
        let snap = snapshot(Some(&f), &menus);
        assert_eq!(snap.focus_pid, 123);
        assert_eq!(snap.app_id, "App");
        assert_eq!(snap.menu_service, "org.example.App");
        assert!(snap.menu_path.is_some());
    }

    #[test]
    fn focus_without_matching_menu_keeps_appid_drops_menu() {
        let menus = MenuMap::default();
        let f = focus(99, "Firefox");
        let snap = snapshot(Some(&f), &menus);
        assert_eq!(snap.app_id, "Firefox");
        assert!(snap.menu_service.is_empty());
        assert!(snap.menu_path.is_none());
    }
}
