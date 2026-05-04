//! Bridge configuration. Loaded from
//! `$XDG_CONFIG_HOME/noctalia-appmenu-bridge/config.toml` (default
//! `~/.config/noctalia-appmenu-bridge/config.toml`), or from the path
//! provided via `--config`.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default trail-edge debounce on niri focus-changed events.
///
/// 75 ms is the empirical sweet spot from ADR-0009: smooth UX during
/// Alt-Tab while still feeling responsive (humans perceive ≥ ~100 ms
/// as a delay).
pub const FOCUS_DEBOUNCE_DEFAULT_MS: u64 = 75;

/// Default trail-edge debounce on registrar churn (e.g. KDE
/// `KMainWindow` rebuilds). 250 ms covers the multi-emit window we
/// observed on kate cold-start without making focus-driven menu
/// updates feel sluggish.
pub const REGISTRAR_DEBOUNCE_DEFAULT_MS: u64 = 250;

/// Default D-Bus service name we publish under. Constant across
/// releases — the QML widget hard-codes it.
pub const PUBLISH_SERVICE_DEFAULT: &str = "org.noctalia.AppMenu";

/// Default D-Bus object path for the active proxy. Constant across
/// releases — the QML widget hard-codes it.
pub const PUBLISH_PATH_DEFAULT: &str = "/org/noctalia/AppMenu/Active";

/// Bridge runtime configuration. All fields have sensible defaults;
/// users override via `~/.config/noctalia-appmenu-bridge/config.toml`
/// or the `--config` flag.
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
            focus_debounce_ms: FOCUS_DEBOUNCE_DEFAULT_MS,
            registrar_debounce_ms: REGISTRAR_DEBOUNCE_DEFAULT_MS,
            niri_binary: PathBuf::from("niri"),
            publish_service: PUBLISH_SERVICE_DEFAULT.to_string(),
            publish_path: PUBLISH_PATH_DEFAULT.to_string(),
        }
    }
}

impl Config {
    /// Load the bridge configuration. If `explicit_path` is `None`, falls
    /// back to `default_config_path()`. Returns the in-memory defaults if
    /// the resolved path does not exist on disk.
    pub fn load(explicit_path: Option<&Path>) -> Result<Self> {
        let path = match explicit_path {
            Some(p) => p.to_path_buf(),
            None => default_config_path()?,
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;

        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}

/// Resolve the user's preferred config path. Order:
///
/// 1. `$XDG_CONFIG_HOME/noctalia-appmenu-bridge/config.toml`
/// 2. `$HOME/.config/noctalia-appmenu-bridge/config.toml`
///
/// Returns an error if neither environment variable is set; refuses to
/// silently fall back to a world-writable directory like `/tmp`.
fn default_config_path() -> Result<PathBuf> {
    let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err(anyhow!(
            "Neither $XDG_CONFIG_HOME nor $HOME is set. \
             Pass --config <path> explicitly."
        ));
    };
    Ok(base.join("noctalia-appmenu-bridge").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_constants_are_finite() {
        let cfg = Config::default();
        assert_eq!(cfg.focus_debounce_ms, FOCUS_DEBOUNCE_DEFAULT_MS);
        assert_eq!(cfg.registrar_debounce_ms, REGISTRAR_DEBOUNCE_DEFAULT_MS);
        assert_eq!(cfg.publish_service, PUBLISH_SERVICE_DEFAULT);
        assert_eq!(cfg.publish_path, PUBLISH_PATH_DEFAULT);
    }

    #[test]
    fn load_missing_returns_defaults() {
        let cfg = Config::load(Some(Path::new("/nonexistent/path"))).unwrap();
        assert_eq!(cfg.focus_debounce_ms, FOCUS_DEBOUNCE_DEFAULT_MS);
    }

    #[test]
    fn load_valid_toml_overrides_defaults() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "focus_debounce_ms = 42").unwrap();
        let cfg = Config::load(Some(f.path())).unwrap();
        assert_eq!(cfg.focus_debounce_ms, 42);
        // Other fields remain at defaults.
        assert_eq!(cfg.registrar_debounce_ms, REGISTRAR_DEBOUNCE_DEFAULT_MS);
    }

    #[test]
    fn load_malformed_toml_errors() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "this is not = valid \"toml\"").unwrap();
        assert!(Config::load(Some(f.path())).is_err());
    }
}
