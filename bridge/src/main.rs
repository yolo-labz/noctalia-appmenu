//! noctalia-appmenu-bridge entry point.
//!
//! Wires the four subsystems together:
//!
//! - `niri`: subscribe to niri-IPC's event-stream, expose a focus-pid feed.
//! - `registrar`: subscribe to com.canonical.AppMenu.Registrar, expose a
//!   pid → (busName, objectPath) map keyed by D-Bus connection PID.
//! - `active`: produce a debounced (focus_pid, menu_address) signal by
//!   joining the two feeds.
//! - `proxy`: own org.noctalia.AppMenu, expose a fixed-path active-app
//!   proxy that mirrors the upstream DBusMenu of the focused app.

use anyhow::Result;
use clap::Parser;
use noctalia_appmenu_bridge::{active, config, niri, proxy, registrar};
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(version, about = "noctalia-appmenu sidecar bridge", long_about = None)]
struct Cli {
    /// Run in foreground (no systemd-notify), log to stderr.
    #[arg(long)]
    foreground: bool,

    /// Print version JSON for telemetry / verify scripts.
    #[arg(long)]
    version_json: bool,

    /// Path to bridge config (TOML). Default: $XDG_CONFIG_HOME/noctalia-appmenu-bridge/config.toml
    #[arg(long)]
    config: Option<std::path::PathBuf>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.version_json {
        println!(
            r#"{{"name":"noctalia-appmenu-bridge","version":"{}","commit":"{}"}}"#,
            env!("CARGO_PKG_VERSION"),
            option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")
        );
        return Ok(());
    }

    init_tracing(cli.foreground);

    let cfg = config::Config::load(cli.config.as_deref())?;
    info!(?cfg, "starting noctalia-appmenu-bridge");

    // Connect to the user session bus — the bridge is a per-user daemon.
    let conn = zbus::Connection::session().await?;

    // Subsystems run as cancellation-safe tasks; the main task waits
    // for SIGTERM / SIGINT and signals all of them to drain.
    let (focus_tx, focus_rx) = tokio::sync::watch::channel(None);
    let (menus_tx, menus_rx) = tokio::sync::watch::channel(registrar::MenuMap::default());
    let (active_tx, active_rx) = tokio::sync::watch::channel(active::ActiveSnapshot::empty());

    let niri_task = tokio::spawn(niri::run(focus_tx, cfg.clone()));
    let registrar_task = tokio::spawn(registrar::run(conn.clone(), menus_tx, cfg.clone()));
    let active_task = tokio::spawn(active::run(focus_rx, menus_rx, active_tx, cfg.clone()));
    let proxy_task = tokio::spawn(proxy::run(conn.clone(), active_rx, cfg.clone()));

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    tokio::select! {
        _ = sigterm.recv() => warn!("SIGTERM — shutting down"),
        _ = sigint.recv() => warn!("SIGINT — shutting down"),
        r = niri_task => warn!(?r, "niri task exited unexpectedly"),
        r = registrar_task => warn!(?r, "registrar task exited unexpectedly"),
        r = active_task => warn!(?r, "active task exited unexpectedly"),
        r = proxy_task => warn!(?r, "proxy task exited unexpectedly"),
    }

    Ok(())
}

fn init_tracing(foreground: bool) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("noctalia_appmenu_bridge=info,zbus=warn"));

    let layer = if foreground {
        fmt::layer().with_ansi(true).boxed()
    } else {
        fmt::layer().json().with_ansi(false).boxed()
    };

    tracing_subscriber::registry().with(env_filter).with(layer).init();
}
