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

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use noctalia_appmenu_bridge::focus::FocusSink;
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
        /// niri window id to focus *before* invoking `DoAction(0)`.
        /// Defaults to 0 (no pre-focus) for back-compat. When > 0, the
        /// subcommand sends `niri msg action focus-window --id <id>`
        /// via niri-ipc and sleeps `focus_settle_ms` before the click,
        /// so multi-window apps (Firefox in particular) route the
        /// action to the captured window rather than to whichever
        /// window has app-internal focus at click time. See issue #109.
        #[arg(long, default_value_t = 0)]
        winid: u64,
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
    if let Some(Cmd::AtspiClick {
        service,
        path,
        winid,
    }) = cli.cmd.as_ref()
    {
        return run_atspi_click(service, path, *winid).await;
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

    // Persistent AT-SPI connection holder (FR-006). Cloned into the
    // active loop (via proxy::run) and the IsEnabled watcher; both
    // share the same lazy-filled `zbus::Connection`. Invalidated by
    // `watch_a11y_status` when the a11y bus restarts.
    let atspi_client = atspi::AtspiClient::new();

    // Subsystems run as cancellation-safe tasks; the main task waits
    // for SIGTERM / SIGINT and signals all of them to drain. Post-
    // ADR-0024 the DBusMenu/Registrar substrate is retired; the bridge
    // walks AT-SPI directly inside the proxy task, so there is no
    // separate menu-map feed.
    let (focus_tx, focus_rx) = tokio::sync::watch::channel(None);
    let (active_tx, active_rx) = tokio::sync::watch::channel(active::ActiveSnapshot::empty());

    // The `FocusSink` trait is the abstraction door — at v1.0.0 the
    // only implementor is `NiriFocusSink` (constitution principle I).
    // Hyprland / Sway / KWin implementors will slot in here in v2
    // without churning the rest of main.rs.
    let niri_task = tokio::spawn(niri::NiriFocusSink::new().run(focus_tx, cfg.clone()));
    let active_task = tokio::spawn(active::run(focus_rx, active_tx, cfg.clone()));
    let proxy_task = tokio::spawn(proxy::run(
        conn.clone(),
        atspi_client.clone(),
        active_rx,
        cfg.clone(),
    ));
    // FR-005: re-flip IsEnabled + invalidate the AT-SPI cache when
    // at-spi2-core restarts. Best-effort task — failures are logged
    // and the loop retries, never fatal.
    let a11y_watcher_task = tokio::spawn(atspi::watch_a11y_status(atspi_client));

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    // niri / active / proxy failures abort the bridge.
    // SIGTERM/SIGINT are graceful shutdowns (exit 0).
    // The a11y_watcher_task is best-effort; we drop its handle into a
    // discard binding so it stays scheduled but its exit does not
    // tear the bridge down.
    let _a11y_watcher_handle = a11y_watcher_task;
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
///
/// FR-007 stale handling: when `do_action` surfaces
/// [`atspi::MenuError::Stale`] we signal the long-running bridge to
/// re-walk the focused app (`org.noctalia.AppMenu.Active.RefreshActive`)
/// and exit with status 2 + a `MenuError::Stale {...}` line on stderr.
/// The QML widget interprets exit-2 as "click missed, snapshot will
/// repaint shortly" and re-renders against the refreshed `active.json`.
async fn run_atspi_click(service: &str, path: &str, winid: u64) -> Result<()> {
    // Issue #109: pre-focus the captured niri window so multi-window
    // apps route `Action.DoAction(0)` to the correct window. Firefox
    // is the canonical case — `DoAction` on its menu accessibles
    // delegates to whichever Firefox window has internal focus, so
    // without this pre-step *New Tab* on window A's menu can open
    // the tab on window B if focus drifted between popup-open and
    // click.
    //
    // `winid == 0` means "no captured window id" — older plugin
    // builds, synthetic items, or first-focus edge cases. Skip the
    // pre-focus entirely there; the original behaviour is preserved.
    if winid > 0 {
        if let Err(e) = niri_focus_window(winid).await {
            // Best-effort: a failed pre-focus is logged but does NOT
            // block the click. niri may be temporarily unreachable
            // and the DoAction often still works (window already had
            // focus). Surfacing as a hard error would regress every
            // single-window scenario.
            eprintln!("warning: niri focus-window {winid} failed: {e:#}; continuing with click");
        } else {
            // Compositor needs a frame to swap input focus and for
            // Firefox to sync its internal "current window". 30 ms
            // is empirically enough on niri 25.x without becoming
            // user-perceptible latency.
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
    }

    match atspi::do_action(service, path).await {
        Ok(()) => Ok(()),
        Err(err) => {
            if let Some(stale) = err.downcast_ref::<atspi::MenuError>() {
                if let Err(e) = signal_refresh_active().await {
                    // Best-effort: even if the daemon is unreachable
                    // the exit-2 still tells the widget to give up
                    // on this click; the next focus event will
                    // naturally refresh the menu tree.
                    eprintln!(
                        "warning: RefreshActive signal failed: {e:#}; bridge will refresh on next focus event"
                    );
                }
                eprintln!("{stale}");
                std::process::exit(2);
            }
            Err(err)
        }
    }
}

/// Send `niri msg action focus-window --id <winid>` over niri-IPC.
/// Used by [`run_atspi_click`] to align compositor focus with the
/// window whose menu the user is acting on (issue #109).
async fn niri_focus_window(winid: u64) -> Result<()> {
    use niri_ipc::{socket::Socket, Action, Request};

    let mut socket = Socket::connect().context("connecting to niri IPC socket")?;
    let reply = socket
        .send(Request::Action(Action::FocusWindow { id: winid }))
        .context("sending FocusWindow action to niri")?;
    reply
        .map_err(|e| anyhow::anyhow!("niri rejected FocusWindow(id={winid}): {e}"))?;
    Ok(())
}

/// Send a `RefreshActive` D-Bus method call to the running bridge so
/// it re-walks the focused app's AT-SPI tree immediately. Used by the
/// short-lived `atspi-click` subprocess when it detects a stale path.
async fn signal_refresh_active() -> Result<()> {
    let conn = zbus::Connection::session()
        .await
        .context("connecting to session bus for RefreshActive")?;
    conn.call_method(
        Some("org.noctalia.AppMenu"),
        "/org/noctalia/AppMenu/Active",
        Some("org.noctalia.AppMenu.Active"),
        "RefreshActive",
        &(),
    )
    .await
    .context("calling org.noctalia.AppMenu.Active.RefreshActive")?;
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
