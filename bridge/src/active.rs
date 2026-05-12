//! Joins the niri focus stream into a debounced "active app menu"
//! snapshot. The AT-SPI walker fires from inside [`run`] on each
//! debounced focus tick.
//!
//! Per ADR-0024 the v0.2 DBusMenu/Registrar substrate is retired;
//! `menu_service` and `menu_path` on [`ActiveSnapshot`] remain only
//! as backward-compatible empty fields so the proxy's D-Bus property
//! contract stays stable for any consumer that has not yet migrated.

use crate::{config::Config, niri::FocusEvent};
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
    #[must_use]
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

/// Pure reducer — produces an `ActiveSnapshot` from the latest niri
/// focus event. Extracted from `run()` so it can be unit-tested
/// independently of the debounce loop.
///
/// `menu_service` and `menu_path` are always empty/`None` under the
/// AT-SPI substrate (ADR-0024); they survive on the snapshot only as
/// legacy D-Bus property carriers for `proxy.rs` and are scheduled for
/// removal in a coordinated schema-v2 PR.
#[must_use]
pub fn snapshot(focus: Option<&FocusEvent>) -> ActiveSnapshot {
    match focus {
        None => ActiveSnapshot::empty(),
        Some(f) => ActiveSnapshot {
            focus_pid: f.pid,
            app_id: f.app_id.clone(),
            title: f.title.clone(),
            menu_service: String::new(),
            menu_path: None,
        },
    }
}

/// Long-running task: trail-edge-debounce changes on the niri focus
/// channel and publish a fresh `ActiveSnapshot` to `tx`. The AT-SPI
/// walk that turns the focus event into a full menu tree fires
/// downstream of this loop — see [`crate::proxy::run`] / the
/// active-loop in this module's sibling code paths.
///
/// Per ADR-0009 the debounce is trailing-edge: on every focus change
/// we sleep for `cfg.focus_debounce_ms` and then read the latest value
/// via `borrow_and_update()`. A focus change during the sleep is
/// absorbed by the next iteration's `changed()` await.
///
/// Returns only on watch channel close (sender dropped) or error.
pub async fn run(
    mut focus_rx: watch::Receiver<Option<FocusEvent>>,
    tx: watch::Sender<ActiveSnapshot>,
    cfg: Config,
) -> anyhow::Result<()> {
    let debounce = Duration::from_millis(cfg.focus_debounce_ms);

    loop {
        if focus_rx.changed().await.is_err() {
            return Ok(());
        }

        // Wait for stillness before publishing.
        tokio::time::sleep(debounce).await;

        let focus = focus_rx.borrow_and_update().clone();
        let snap = snapshot(focus.as_ref());

        debug!(?snap, "active snapshot");
        let _ = tx.send(snap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus(pid: u32, app: &str) -> FocusEvent {
        FocusEvent {
            winid: 1,
            pid,
            app_id: app.into(),
            title: "t".into(),
        }
    }

    #[test]
    fn no_focus_yields_empty() {
        assert_eq!(snapshot(None), ActiveSnapshot::empty());
    }

    #[test]
    fn focus_populates_pid_app_id_title() {
        let f = focus(123, "App");
        let snap = snapshot(Some(&f));
        assert_eq!(snap.focus_pid, 123);
        assert_eq!(snap.app_id, "App");
        assert_eq!(snap.title, "t");
    }

    #[test]
    fn snapshot_omits_legacy_dbusmenu_fields_post_atspi() {
        // ADR-0024: menu_service / menu_path are AT-SPI-substrate
        // dead fields — kept for proxy D-Bus property carrier only.
        let f = focus(99, "Firefox");
        let snap = snapshot(Some(&f));
        assert!(snap.menu_service.is_empty());
        assert!(snap.menu_path.is_none());
    }
}
