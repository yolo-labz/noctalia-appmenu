//! noctalia-appmenu-bridge entry point.
//!
//! Wires the four subsystems together:
//!
//! - `niri`: subscribe to niri-IPC's event-stream, expose a focus-pid feed.
//! - `registrar`: subscribe to com.canonical.AppMenu.Registrar, expose a
//!   pid → (busName, objectPath) map keyed by D-Bus connection PID.
//! - `active`: produce a debounced (`focus_pid`, `menu_address`) signal by
//!   joining the two feeds.
//! - `proxy`: own org.noctalia.AppMenu, expose a fixed-path active-app
//!   proxy that mirrors the upstream `DBusMenu` of the focused app.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use noctalia_appmenu_bridge::{active, atspi, config, niri, proxy, registrar};
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
    /// Forward a click event to a registered `DBusMenu` item. Invoked
    /// from the QML widget on user click.
    ///
    /// Calls `com.canonical.dbusmenu::Event(itemId, "clicked", "",
    /// timestamp)` on the registered app's menu service. Apps
    /// respond by activating the corresponding menu action — same
    /// effect as if the user had clicked it in-window.
    ///
    /// v0.2 phase D — invoked from QML via Process.spawn.
    Click {
        /// D-Bus bus name (well-known or unique) of the registered app.
        bus_name: String,
        /// Object path of the app's `com.canonical.dbusmenu` service.
        menu_path: String,
        /// Menu item id from the layout returned by `GetLayout`.
        item_id: i32,
    },

    /// AT-SPI click forward (v0.3 substrate). Calls
    /// `org.a11y.atspi.Action.DoAction(0)` on the accessible at the
    /// given (service, path) coordinates — the same pair the QML
    /// widget reads out of `active.json`'s menu tree.
    ///
    /// One-shot subcommand: spawn, call, exit. Bridge daemon stays
    /// up; this short-lived process does the click and goes away.
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
    match cli.cmd {
        Some(Cmd::Click {
            ref bus_name,
            ref menu_path,
            item_id,
        }) => return handle_click(bus_name, menu_path, item_id).await,
        Some(Cmd::AtspiClick {
            ref service,
            ref path,
        }) => return atspi::do_action(service, path).await,
        None => {}
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

    // Selecting on the four task JoinHandles means an unexpected task
    // exit returns from select!. SIGTERM/SIGINT are graceful shutdowns
    // (exit 0). Any task exit is an unrecoverable error — return non-
    // zero so systemd's `Restart=on-failure` re-spawns us.
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
        r = registrar_task => {
            warn!(?r, "registrar task exited unexpectedly");
            anyhow::bail!("registrar task exited: {r:?}")
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

/// CLI `click` subcommand: forward an Event(itemId, "clicked", "",
/// timestamp) call to the registered app's `DBusMenu` service.
///
/// Run as a one-shot child process spawned by the QML widget. We
/// intentionally don't bring up the full bridge runtime — connect,
/// call, exit. The widget gets click responsiveness while the
/// long-running bridge stays focused on its job.
///
/// **Failure modes (all logged + non-fatal exit):**
/// - App disappeared between fetch and click → zbus call returns
///   error; we log + exit non-zero. Widget gets feedback via
///   process exit code (no UX consequence today; v0.2.1 could
///   re-fetch the menu tree on error to remove stale items).
/// - Invalid bus name / object path → parse error at proxy build
///   time; stderr line + non-zero exit.
async fn handle_click(bus_name: &str, menu_path: &str, item_id: i32) -> Result<()> {
    use zbus::zvariant::Value;

    let conn = zbus::Connection::session()
        .await
        .context("connecting to session bus for click")?;

    // Build a one-shot dbusmenu proxy. We can't reuse the
    // dbusmenu.rs proxy directly because it's `trait`-private to
    // that module — but since this is a separate process, we
    // instantiate the same wire interface inline and skip the
    // full module abstraction.
    let proxy_path: zbus::zvariant::ObjectPath<'_> = menu_path
        .try_into()
        .with_context(|| format!("parsing object path {menu_path}"))?;
    let proxy_dest: zbus::names::BusName<'_> = bus_name
        .try_into()
        .with_context(|| format!("parsing bus name {bus_name}"))?;

    let timestamp: u32 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as u32);

    let proxy = zbus::Proxy::new(&conn, proxy_dest, proxy_path, "com.canonical.dbusmenu")
        .await
        .context("building dbusmenu proxy for click")?;

    // dbusmenu Event: (i, s, v, u). The variant data carries
    // optional event payload; "" (empty string) is the documented
    // value for plain clicks.
    proxy
        .call_method("Event", &(item_id, "clicked", Value::from(""), timestamp))
        .await
        .with_context(|| {
            format!("Event({item_id}, \"clicked\") failed — app may have left the bus")
        })?;

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

    tracing_subscriber::registry()
        .with(env_filter)
        .with(layer)
        .init();
}
