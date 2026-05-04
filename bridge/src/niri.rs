//! niri-IPC integration: focus-changed events + windows snapshot.
//!
//! On startup we shell out to `niri msg --json windows` once to seed
//! the `winid -> pid` map. We then long-pipe `niri msg --json
//! event-stream` and update the map (and emit focus changes) as events
//! arrive. ADR-0002 + ADR-0005.

use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct NiriWindow {
    pub id: u64,
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub pid: Option<u32>,
    pub workspace_id: Option<u64>,
    pub is_focused: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "PascalCase")]
enum NiriEvent {
    WindowFocusChanged { id: Option<u64> },
    WindowOpenedOrChanged { window: NiriWindow },
    WindowClosed { id: u64 },
    /// Catch-all so unknown event variants don't crash the stream.
    #[serde(other)]
    Other,
}

/// What we publish for downstream consumers.
#[derive(Debug, Clone, PartialEq)]
pub struct FocusEvent {
    pub winid: u64,
    pub pid: u32,
    pub app_id: String,
    pub title: String,
}

pub async fn run(tx: watch::Sender<Option<FocusEvent>>, cfg: Config) -> Result<()> {
    // Seed the map.
    let mut by_winid = snapshot_windows(&cfg).await.unwrap_or_else(|e| {
        warn!(error=?e, "could not seed winid->window map; proceeding empty");
        HashMap::new()
    });

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
        match serde_json::from_str::<NiriEvent>(line) {
            Ok(NiriEvent::WindowFocusChanged { id: Some(id) }) => {
                if let Some(win) = by_winid.get(&id).cloned() {
                    if let Some(pid) = win.pid {
                        let evt = FocusEvent {
                            winid: id,
                            pid,
                            app_id: win.app_id.unwrap_or_default(),
                            title: win.title.unwrap_or_default(),
                        };
                        debug!(?evt, "focus changed");
                        let _ = tx.send(Some(evt));
                    } else {
                        warn!(winid = id, "focused window has no pid in our map");
                    }
                } else {
                    // Stale map; resync.
                    by_winid = snapshot_windows(&cfg).await.unwrap_or(by_winid);
                }
            }
            Ok(NiriEvent::WindowFocusChanged { id: None }) => {
                let _ = tx.send(None);
            }
            Ok(NiriEvent::WindowOpenedOrChanged { window }) => {
                by_winid.insert(window.id, window);
            }
            Ok(NiriEvent::WindowClosed { id }) => {
                by_winid.remove(&id);
            }
            Ok(NiriEvent::Other) => {}
            Err(e) => {
                warn!(error=?e, line=%line, "could not parse niri event line");
            }
        }
    }

    error!("niri event-stream ended");
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

    let windows: Vec<NiriWindow> = serde_json::from_slice(&out.stdout)
        .context("parsing niri msg windows JSON")?;

    Ok(windows.into_iter().map(|w| (w.id, w)).collect())
}
