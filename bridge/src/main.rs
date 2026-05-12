//! noctalia-appmenu-bridge entry point.
//!
//! Wires the three subsystems together (post-ADR-0024 substrate):
//!
//! - `niri`: subscribe to niri-IPC's event-stream, expose a focus-pid feed
//!   (the only `FocusSink` implementor at v1).
//! - `active`: walk the focused app's AT-SPI menubar on each focus tick,
//!   emit `active.json` + push it via Quickshell IPC.
//! - `proxy`: own org.noctalia.AppMenu, expose a fixed-path active-app
//!   proxy with the focused window's metadata as D-Bus properties.

use anyhow::Result;
use clap::{Parser, Subcommand};
use noctalia_appmenu_bridge::{active, atspi, config, niri, proxy};
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

    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// AT-SPI click forward (v0.3 substrate). Calls
    /// `org.a11y.atspi.Action.DoAction(0)` on the accessible at the
    /// given (service, path) coordinates — the same pair the QML
    /// widget reads out of `active.json`'s menu tree.
    ///
    /// One-shot subcommand: spawn, call, exit. Bridge daemon stays
    /// up; this short-lived process does the click and goes away.
    /// On stale path (the app rebuilt its widget tree between
    /// snapshot and click) the subcommand exits with code 2 +
    /// stderr `MenuError::Stale {...}` — see spec 005 FR-007.
    AtspiClick {
        /// AT-SPI bus name (e.g. `:1.42` — unique connection).
        service: String,
        /// AT-SPI object path (e.g. `/org/a11y/atspi/accessible/12`).
        path: String,
    },
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

    // CLI subcommands run in their own short-lived process — no
    // tracing setup, no daemon machinery, no D-Bus listener. Just
    // do the call and exit.
    if let Some(Cmd::AtspiClick { service, path }) = cli.cmd.as_ref() {
        return run_atspi_click(service, path).await;
    }

    init_tracing(cli.foreground);

    let cfg = config::Config::load(cli.config.as_deref())?;
    info!(?cfg, "starting noctalia-appmenu-bridge");

    // Flip `org.a11y.Status.IsEnabled = true` so Qt apps register
    // their accessible trees on the a11y bus. niri ships no AT
    // (Orca etc), so nobody else flips it; without this, the
    // registry stays empty and our walker finds nothing.
    if let Err(e) = atspi::enable_a11y().await {
        warn!(error = ?e, "atspi enable failed — qt apps may not expose menus");
    } else {
        info!("atspi a11y bus enabled");
    }

    // Connect to the user session bus — the bridge is a per-user daemon.
    let conn = zbus::Connection::session().await?;

    // Subsystems run as cancellation-safe tasks; the main task waits
    // for SIGTERM / SIGINT and signals all of them to drain. Post-
    // ADR-0024 the DBusMenu/Registrar substrate is retired; the bridge
    // walks AT-SPI directly inside the active task, so there is no
    // separate menu-map feed.
    let (focus_tx, focus_rx) = tokio::sync::watch::channel(None);
    let (active_tx, active_rx) = tokio::sync::watch::channel(active::ActiveSnapshot::empty());

    let niri_task = tokio::spawn(niri::run(focus_tx, cfg.clone()));
    let active_task = tokio::spawn(active::run(focus_rx, active_tx, cfg.clone()));
    let proxy_task = tokio::spawn(proxy::run(conn.clone(), active_rx, cfg.clone()));

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    // niri / active / proxy failures abort the bridge.
    // SIGTERM/SIGINT are graceful shutdowns (exit 0).
    tokio::select! {
        _ = sigterm.recv() => {
            warn!("SIGTERM — shutting down");
            Ok(())
        }
        _ = sigint.recv() => {
            warn!("SIGINT — shutting down");
            Ok(())
        }
        r = niri_task => {
            warn!(?r, "niri task exited unexpectedly");
            anyhow::bail!("niri task exited: {r:?}")
        }
        r = active_task => {
            warn!(?r, "active task exited unexpectedly");
            anyhow::bail!("active task exited: {r:?}")
        }
        r = proxy_task => {
            warn!(?r, "proxy task exited unexpectedly");
            anyhow::bail!("proxy task exited: {r:?}")
        }
    }
}

/// CLI `atspi-click` subcommand: forward the click to the AT-SPI
/// accessible at `(service, path)`. Delegates to [`atspi::do_action`].
async fn run_atspi_click(service: &str, path: &str) -> Result<()> {
    atspi::do_action(service, path).await
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

    tracing_subscriber::registry()
        .with(env_filter)
        .with(layer)
        .init();
}
