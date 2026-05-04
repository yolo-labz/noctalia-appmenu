//! Bridge configuration. Loaded from
//! `$XDG_CONFIG_HOME/noctalia-appmenu-bridge/config.toml` (default
//! `~/.config/noctalia-appmenu-bridge/config.toml`), or from the path
//! provided via `--config`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Trail-edge debounce on niri focus-changed events. ADR-0009.
    pub focus_debounce_ms: u64,

    /// Trail-edge debounce on registrar churn. ADR-0009.
    pub registrar_debounce_ms: u64,

    /// niri-IPC binary. Resolved at startup, not per call.
    pub niri_binary: PathBuf,

    /// D-Bus service name we publish under. Constant across releases —
    /// the QML widget hard-codes it.
    pub publish_service: String,

    /// D-Bus object path of the active proxy. Constant across releases.
    pub publish_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            focus_debounce_ms: 75,
            registrar_debounce_ms: 250,
            niri_binary: PathBuf::from("niri"),
            publish_service: "org.noctalia.AppMenu".to_string(),
            publish_path: "/org/noctalia/AppMenu/Active".to_string(),
        }
    }
}

impl Config {
    pub fn load(explicit_path: Option<&Path>) -> Result<Self> {
        let path = match explicit_path {
            Some(p) => p.to_path_buf(),
            None => default_config_path(),
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;

        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}

fn default_config_path() -> PathBuf {
    let xdg = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .ok()
        })
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    xdg.join("noctalia-appmenu-bridge").join("config.toml")
}
