//! AT-SPI menubar walker — v0.3 substrate (Path A).
//!
//! Replaces v0.2's DBusMenu/Registrar approach. The `DBusMenu` protocol
//! requires apps to call `RegisterWindow` against a registrar
//! service — but Qt6's auto-registration only fires on compositors
//! implementing `org_kde_kwin_appmenu_manager` (`KWin` only). niri,
//! Hyprland, Sway, COSMIC: none implement it. Result: no Qt app on
//! niri ever registered against our v0.2 bridge, regardless of
//! correctness.
//!
//! AT-SPI is the cross-toolkit substrate that already works:
//!
//! - Qt apps load `qtatspi` plugin at `QApplication` startup when
//!   `QT_ACCESSIBILITY=1` is set (NixOS module ships this).
//! - Qt's `QMenuBar` is exposed under `Role::MenuBar` automatically.
//! - GTK apps expose menus via ATK→AT-SPI without any extra config.
//! - Anki, Okular, Firefox, GIMP all surface menubars identically.
//! - No protocol cooperation required from the compositor.
//!
//! ## Connection topology
//!
//! AT-SPI lives on its own dedicated D-Bus bus (NOT the session bus)
//! to keep accessibility traffic isolated. To connect:
//!
//! 1. Query `org.a11y.Bus` on the SESSION bus for the a11y bus
//!    address via `GetAddress()`.
//! 2. Connect to that address (typically a UNIX socket like
//!    `unix:abstract=/tmp/at-spi2-bus-XXXXX/socket`).
//! 3. Use the registry root at well-known service
//!    `org.a11y.atspi.Registry`, path
//!    `/org/a11y/atspi/accessible/root`. Its children are the
//!    registered Application objects, one per running a11y-aware
//!    app.
//!
//! ## Walking the tree
//!
//! `org.a11y.atspi.Accessible` interface methods (the only ones we
//! need for v0.3.0):
//!
//! - `GetChildAtIndex(i: i32) → (s, o)` — returns (busName, path) of
//!   the i-th child. AT-SPI uses pairs because every accessible
//!   object lives on its own object path WITHIN a single
//!   application's bus connection.
//! - `Property: ChildCount (i)` — int32 count of children.
//! - `Property: Name (s)` — display name (e.g. "File", "Edit").
//! - `GetRole() → u`. WIRE-LEVEL role enum from at-spi2-core's
//!   `atspi-constants.h` (NOT pyatspi's older numeric mapping —
//!   verified live against okular 26.04 + Qt 6.11):
//!   - `CHECK_MENU_ITEM = 8`
//!   - `MENU = 33`
//!   - `MENU_BAR = 34`
//!   - `MENU_ITEM = 35`
//!   - `RADIO_MENU_ITEM = 45`
//!   - `SEPARATOR = 50`
//!   - `TEAR_OFF_MENU_ITEM = 60`
//! - `GetState() → au` — array of state ints. Useful: ENABLED=20,
//!   VISIBLE=37, FOCUSABLE=10.
//! - `GetApplication() → (s, o)` — returns the owning app's
//!   accessible (root of the per-app subtree).
//!
//! `org.a11y.atspi.Action` interface (for click forwarding):
//!
//! - `DoAction(i: i32) → b` — invoke the i-th action. Index 0 is
//!   "click" by convention (verified against Qt's qtatspi). Returns
//!   true on success.
//! - `Property: NActions (i)` — count.
//!
//! ## PID matching
//!
//! niri's `WindowFocusChanged` event gives us a PID. AT-SPI doesn't
//! key on PID directly. We discover the matching app via:
//!
//! 1. Walk Registry root's children (each is an Application).
//! 2. For each, resolve its bus name to a PID via the a11y bus's
//!    `org.freedesktop.DBus.GetConnectionUnixProcessID(name)`.
//! 3. Match against niri's focused PID. First hit wins.

use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::{
    proxy,
    zvariant::{ObjectPath, OwnedObjectPath},
    Connection,
};

/// AT-SPI **wire-level** role IDs from at-spi2-core's
/// `atspi-constants.h` (`AtspiRole` enum). These match what
/// `org.a11y.atspi.Accessible.GetRole()` returns over D-Bus —
/// stable across at-spi2-core releases since the enum is part of
/// the protocol contract.
///
/// Verified live 2026-05-06 against okular 26.04 + Qt 6.11 menubar:
/// menubar→34, menu items→35, separator→50.
mod role {
    pub const CHECK_MENU_ITEM: u32 = 8;
    pub const MENU: u32 = 33;
    pub const MENU_BAR: u32 = 34;
    /// Top-level menubar children in Qt are `MENU_ITEM` (not MENU);
    /// the actual popup MENU is one level below. We don't dispatch
    /// on this constant directly (count > 0 catches submenu shape),
    /// but keep it documented for the role-name table.
    #[allow(dead_code)]
    pub const MENU_ITEM: u32 = 35;
    pub const RADIO_MENU_ITEM: u32 = 45;
    pub const SEPARATOR: u32 = 50;
    #[allow(dead_code)]
    pub const TEAR_OFF_MENU_ITEM: u32 = 60;
}

/// Maximum tree depth we'll walk looking for a `MenuBar`. Some apps
/// nest menubars under deep window/toolbar hierarchies. Cap to
/// prevent runaway walks on malformed trees.
const MAX_FIND_DEPTH: u32 = 8;

/// v1.0.9 — self-learning no-menubar skip, replacing the hardcoded
/// `KNOWN_NO_MENUBAR_APPS` list (v1.0.6..v1.0.8). The skip set now
/// populates itself from observed walk outcomes, keyed by `app_id`,
/// so terminals/X11/Chrome are skipped automatically with no per-app
/// id maintained in source.
///
/// Why a list existed at all (Pedro field report 16/05/2026): focusing
/// xwayland-satellite-hosted X11 apps and terminals drained the full
/// `FETCH_BUDGET` on every focus event because they are not on the
/// a11y bus — the per-connection PID scan finds no match only after
/// exhausting every registered app. That made the bar feel frozen.
///
/// The learned mechanism reproduces the list's protection from the
/// *outcome* of the walk, not a name:
///   - A walk that returns no menubar *expensively* (≥ `EXPENSIVE_WALK`)
///     is the not-on-bus case (terminals, xwayland, or an app whose a11y
///     is currently disabled). Re-walking re-freezes the bar, so the
///     verdict is held for a long `EXPENSIVE_RECHECK_TTL` — but NOT
///     permanently: an off-bus condition is recoverable (Firefox's
///     `accessibility.force_disabled` 1→0, a late-starting AT, an
///     a11y-bus restart), and a permanent verdict strands the app on the
///     desktop fallback forever (issue #174). It also clears immediately
///     when the a11y bus is observed restarting (`watch_a11y_status`).
///   - A walk that returns no menubar *cheaply* is the on-bus
///     no-`MENU_BAR` case (Chrome's hamburger menu, GTK4 popover-only
///     apps). Cheap to re-confirm, so the verdict expires after
///     `RECHECK_TTL` — a lazily-built menubar self-heals on re-walk.
///   - A walk that finds a real menubar (`forget`) drops any verdict,
///     so an app that gained a menu is never skipped.
///
/// Transient `Err` walks record nothing (not a "no menubar" signal).
mod learned_skip {
    use std::collections::{HashMap, HashSet};
    use std::sync::{LazyLock, Mutex};
    use std::time::{Duration, Instant};

    /// A no-menubar walk slower than this drained most of the
    /// `FETCH_BUDGET`: the app is not on the a11y bus. Healthy apps
    /// answer in < 50 ms; Chrome's depth-8 DFS over a populated tree
    /// is well under this. 750 ms cleanly separates "DFS one app" from
    /// "scanned every connection and gave up / hung". Heuristic knob:
    /// a cheap app misclassified expensive only loses lazy self-heal
    /// (harmless — it has no menu); the reverse cannot happen because
    /// not-on-bus apps have no menu to lazily build.
    const EXPENSIVE_WALK: Duration = Duration::from_millis(750);

    /// How long a *cheap* no-menubar verdict suppresses re-walks before
    /// the next focus re-walks once and re-learns (on-bus app, lazily
    /// built menu — GTK4 popover, Chrome hamburger).
    const RECHECK_TTL: Duration = Duration::from_secs(300);

    /// How long an *expensive* (not-on-bus) verdict suppresses re-walks.
    /// Long, because re-walking an off-bus app drains `FETCH_BUDGET` and
    /// briefly stalls the bar — but FINITE, because off-bus is recoverable
    /// (issue #174: a permanent verdict stranded Firefox on the desktop
    /// fallback after its a11y was enabled, until a bridge restart). 30 min
    /// bounds the worst-case re-stall to ~twice an hour per off-bus app.
    const EXPENSIVE_RECHECK_TTL: Duration = Duration::from_secs(1800);

    /// How long an *expensive* verdict suppresses re-walks for an app that
    /// has resolved a real menubar before this process (`RESOLVED_ONCE`).
    /// Short: a known-GUI app whose menu is momentarily missing is almost
    /// always a cold-start a11y race (Firefox after a restart instantiates
    /// its AT-SPI tree a beat late; the bridge focuses it in that window,
    /// learns an expensive skip, and the menu "vanishes" until the TTL).
    /// 30 s lets such an app recover fast once it re-registers, while a
    /// terminal — which never resolved a menubar — keeps the long
    /// `EXPENSIVE_RECHECK_TTL` and never re-stalls the bar.
    const EXPENSIVE_RESOLVED_TTL: Duration = Duration::from_secs(30);

    struct Verdict {
        learned_at: Instant,
        expensive: bool,
    }

    static SKIP: LazyLock<Mutex<HashMap<String, Verdict>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    /// App-ids that have resolved a real menubar at least once this
    /// process. Lets [`should_skip`] tell a known-GUI app whose menu is
    /// temporarily missing (cold-start race) from a terminal that never
    /// had one: the former recovers from an expensive skip in
    /// `EXPENSIVE_RESOLVED_TTL`, the latter in `EXPENSIVE_RECHECK_TTL`.
    static RESOLVED_ONCE: LazyLock<Mutex<HashSet<String>>> =
        LazyLock::new(|| Mutex::new(HashSet::new()));

    fn resolved_once(app_id: &str) -> bool {
        RESOLVED_ONCE
            .lock()
            .map(|s| s.contains(app_id))
            .unwrap_or(false)
    }

    /// Recheck cadence for a verdict by cost class. Pure fn so
    /// [`skip_decision`] stays a trivial age compare and the boundary is
    /// unit-testable without sleeping any TTL.
    const fn ttl_for(expensive: bool, resolved_once: bool) -> Duration {
        if !expensive {
            RECHECK_TTL
        } else if resolved_once {
            EXPENSIVE_RESOLVED_TTL
        } else {
            EXPENSIVE_RECHECK_TTL
        }
    }

    /// Pure verdict-currency decision, split out so tests can probe the
    /// TTL boundary without sleeping. A verdict is honoured only while it
    /// is younger than its cost-class TTL — NEVER permanently (issue #174;
    /// see the constitution's liveness-cache self-heal invariant). An app
    /// that has resolved a menubar before uses the short
    /// `EXPENSIVE_RESOLVED_TTL`, so a cold-start race self-heals in seconds.
    fn skip_decision(expensive: bool, resolved_once: bool, age: Duration) -> bool {
        age < ttl_for(expensive, resolved_once)
    }

    /// Pure cost classifier, split out for the same reason.
    fn classify_expensive(walk: Duration) -> bool {
        walk >= EXPENSIVE_WALK
    }

    /// Tier-1 fast reject: has this `app_id` been learned to have no
    /// usable menubar, and is that verdict still current?
    pub fn should_skip(app_id: &str) -> bool {
        let resolved = resolved_once(app_id);
        let Ok(map) = SKIP.lock() else {
            return false;
        };
        let Some(v) = map.get(app_id) else {
            return false;
        };
        let age = v.learned_at.elapsed();
        let skip = skip_decision(v.expensive, resolved, age);
        if skip && resolved {
            // A known-menubar app being skipped is the surprising case
            // worth surfacing ("Firefox menu vanished"); terminals being
            // skipped is normal and stays quiet. Logs which app + how long,
            // so a recurrence is diagnosable from the journal.
            tracing::info!(
                app_id,
                expensive = v.expensive,
                age_s = age.as_secs(),
                ttl_s = ttl_for(v.expensive, resolved).as_secs(),
                "learned-skip honoured for a known-menubar app — serving \
                 desktop fallback; self-heals at ttl (cold-start race?)"
            );
        }
        skip
    }

    /// Record that a walk for `app_id` found no menubar. `walk` is the
    /// wall-clock cost of that walk; see `EXPENSIVE_WALK`.
    pub fn record_negative(app_id: &str, walk: Duration) {
        if let Ok(mut map) = SKIP.lock() {
            map.insert(
                app_id.to_string(),
                Verdict {
                    learned_at: Instant::now(),
                    expensive: classify_expensive(walk),
                },
            );
        }
    }

    /// A walk found a real menubar for `app_id` — drop any verdict so
    /// the app is never skipped again, and remember it as a known-menubar
    /// app so a future cold-start expensive skip recovers in
    /// `EXPENSIVE_RESOLVED_TTL` rather than the full terminal TTL.
    pub fn forget(app_id: &str) {
        if let Ok(mut map) = SKIP.lock() {
            map.remove(app_id);
        }
        if let Ok(mut set) = RESOLVED_ONCE.lock() {
            set.insert(app_id.to_string());
        }
    }

    /// Drop every *expensive* (not-on-bus) verdict. Called when the a11y
    /// bus is observed restarting ([`watch_a11y_status`]): off-bus apps may
    /// have re-registered, so their stale verdicts must not suppress the
    /// next walk. Cheap verdicts keep their own short TTL.
    pub fn clear_expensive() {
        if let Ok(mut map) = SKIP.lock() {
            map.retain(|_, v| !v.expensive);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn expensive_walk_is_classified_expensive() {
            assert!(classify_expensive(Duration::from_millis(2000)));
            assert!(classify_expensive(EXPENSIVE_WALK)); // boundary inclusive
            assert!(!classify_expensive(Duration::from_millis(50)));
            assert!(!classify_expensive(Duration::from_millis(749)));
        }

        // Serializes the tests that mutate the global SKIP map across the
        // expensive class (clear_expensive is global), preventing flakes
        // under cargo's parallel test runner.
        static SKIP_GUARD: Mutex<()> = Mutex::new(());

        #[test]
        fn expensive_verdict_is_bounded_not_permanent() {
            // Issue #174: a never-resolved (terminal) expensive verdict must
            // self-heal, never strand.
            assert!(skip_decision(true, false, Duration::from_secs(10)));
            assert!(skip_decision(
                true,
                false,
                EXPENSIVE_RECHECK_TTL - Duration::from_secs(1)
            ));
            assert!(!skip_decision(true, false, EXPENSIVE_RECHECK_TTL));
            assert!(!skip_decision(true, false, Duration::from_secs(86_400)));
        }

        #[test]
        fn expensive_ttl_is_finite_and_outlives_cheap() {
            assert!(ttl_for(true, false) > ttl_for(false, false));
        }

        #[test]
        fn resolved_app_recovers_from_expensive_skip_fast() {
            // A known-menubar app (Firefox after a restart) uses the short
            // resolved TTL, so a cold-start expensive skip clears in seconds
            // instead of the full terminal TTL.
            assert!(ttl_for(true, true) < ttl_for(true, false));
            assert_eq!(ttl_for(true, true), EXPENSIVE_RESOLVED_TTL);
            assert!(skip_decision(true, true, Duration::from_secs(5)));
            assert!(!skip_decision(true, true, EXPENSIVE_RESOLVED_TTL));
        }

        #[test]
        fn cheap_verdict_expires_after_ttl() {
            assert!(skip_decision(false, false, Duration::from_secs(10)));
            assert!(!skip_decision(false, false, RECHECK_TTL));
            assert!(!skip_decision(
                false,
                false,
                RECHECK_TTL + Duration::from_secs(1)
            ));
        }

        #[test]
        fn unknown_app_is_not_skipped() {
            assert!(!should_skip("org.test.never-seen-app-id"));
        }

        #[test]
        fn cheap_negative_skips_then_forget_clears() {
            let app = "org.test.cheap-app";
            record_negative(app, Duration::from_millis(40));
            assert!(should_skip(app), "fresh cheap verdict should skip");
            forget(app);
            assert!(!should_skip(app), "forget should clear the verdict");
        }

        #[test]
        fn expensive_negative_skips_immediately() {
            let _g = SKIP_GUARD.lock().unwrap();
            let app = "org.test.expensive-app";
            record_negative(app, Duration::from_millis(2400));
            assert!(should_skip(app));
            forget(app);
        }

        #[test]
        fn clear_expensive_drops_only_expensive_verdicts() {
            let _g = SKIP_GUARD.lock().unwrap();
            let cheap = "org.test.cheap-survives-clear";
            let pricey = "org.test.expensive-cleared";
            record_negative(cheap, Duration::from_millis(40));
            record_negative(pricey, Duration::from_millis(2400));
            assert!(should_skip(cheap));
            assert!(should_skip(pricey));
            clear_expensive();
            assert!(should_skip(cheap), "cheap verdict survives clear_expensive");
            assert!(
                !should_skip(pricey),
                "expensive verdict cleared (issue #174 self-heal)"
            );
            forget(cheap);
        }
    }
}

pub use learned_skip::{
    clear_expensive as clear_expensive_skips, forget as forget_menubar,
    record_negative as record_negative_walk, should_skip as should_skip_walk,
};

/// v1.0.6 — TTL-bounded cache of AT-SPI menu walks. v1.0.25 widened
/// the key from `(app_id, pid)` to `(app_id, pid, winid)`: a single
/// process can own several windows (Firefox, okular, LibreOffice) whose
/// menus differ, so a pid-only key served one window's menu for another
/// until the TTL expired (the cache half of the wrong-window-routing
/// bug — see ADR-0030). `winid` is niri's stable per-session window id.
///
/// Re-focusing the same window within `MENU_CACHE_TTL` returns the
/// cached menu instantly (no D-Bus traffic at all).
///
/// Cache stores BOTH `Some(menu)` (positive) and `None` (negative —
/// app exposes nothing) so we don't re-walk apps we've already
/// confirmed to have no menubar.
mod menu_cache {
    use super::MenuItem;
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};
    use std::time::{Duration, Instant};

    pub const MENU_CACHE_TTL: Duration = Duration::from_secs(30);

    struct Entry {
        menu: Option<MenuItem>,
        fetched_at: Instant,
    }

    type Key = (String, u32, u64);
    static CACHE: LazyLock<Mutex<HashMap<Key, Entry>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    /// Returns `Some(cached_value)` (where the inner `Option<MenuItem>`
    /// is the actual cached result) on hit; `None` on miss or stale.
    pub fn get(app_id: &str, pid: u32, winid: u64) -> Option<Option<MenuItem>> {
        let cache = CACHE.lock().ok()?;
        let entry = cache.get(&(app_id.to_string(), pid, winid))?;
        if entry.fetched_at.elapsed() > MENU_CACHE_TTL {
            return None;
        }
        Some(entry.menu.clone())
    }

    pub fn put(app_id: &str, pid: u32, winid: u64, menu: Option<MenuItem>) {
        if let Ok(mut cache) = CACHE.lock() {
            cache.insert(
                (app_id.to_string(), pid, winid),
                Entry {
                    menu,
                    fetched_at: Instant::now(),
                },
            );
        }
    }

    /// Drop a specific entry (e.g. after a `MenuError::Stale` re-walk).
    pub fn invalidate(app_id: &str, pid: u32, winid: u64) {
        if let Ok(mut cache) = CACHE.lock() {
            cache.remove(&(app_id.to_string(), pid, winid));
        }
    }
}

pub use menu_cache::{
    get as cached_menu_for_pid, invalidate as invalidate_menu_cache, put as cache_menu_for_pid,
};

/// Maximum recursion depth for fetching menu items once we've
/// found a `MenuBar`. v1.0.6 lowered from 6 to 3.
///
/// Real-world menus cap at depth 3 (e.g. Firefox History > Recently
/// Closed Windows > <window-title>; shadPS4 View > Game List Mode >
/// [List, Grid, Flat]). Walking past 3 wastes D-Bus round-trips and
/// inflates active.json — Pedro reports 500ms+ Firefox walks at
/// depth=6.
const MAX_FETCH_DEPTH: u32 = 3;

/// Minimum subset of `org.a11y.atspi.Accessible` we use. Methods
/// that return `(busName, path)` pairs are the AT-SPI way of
/// representing references across the per-app subtrees that live
/// on different bus connections within the same a11y bus.
///
/// `Name` and `ChildCount` ARE properties on the wire, but we read
/// them via `org.freedesktop.DBus.Properties.Get` (helper functions
/// below) rather than `#[zbus(property)]` to avoid `GetAll` caching —
/// AT-SPI accessibles don't all return a populated `a{sv}` for
/// `GetAll`, which causes zbus's cache fill to error out.
#[proxy(
    interface = "org.a11y.atspi.Accessible",
    default_path = "/org/a11y/atspi/accessible/root"
)]
trait Accessible {
    fn get_child_at_index(&self, idx: i32) -> zbus::Result<(String, OwnedObjectPath)>;

    fn get_role(&self) -> zbus::Result<u32>;

    fn get_role_name(&self) -> zbus::Result<String>;

    /// Bitmask of state flags. Wire format is `au` — array of u32.
    /// State indices we care about: ENABLED=20, VISIBLE=37.
    fn get_state(&self) -> zbus::Result<Vec<u32>>;
}

/// Read `Name` (string) on an accessible via Properties.Get, in
/// place of `#[zbus(property)]` because the macro forces a
/// Properties.GetAll on first access and AT-SPI accessibles
/// respond with an empty `a{sv}` that zbus rejects as a signature
/// mismatch.
async fn read_name(
    a11y: &Connection,
    service: &str,
    path: &ObjectPath<'_>,
) -> zbus::Result<String> {
    let props = zbus::fdo::PropertiesProxy::builder(a11y)
        .destination(service.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let v = props
        .get("org.a11y.atspi.Accessible".try_into()?, "Name")
        .await?;
    String::try_from(v).map_err(zbus::Error::Variant)
}

/// Read `ChildCount` (i32) on an accessible — same reason as `read_name`.
async fn read_child_count(
    a11y: &Connection,
    service: &str,
    path: &ObjectPath<'_>,
) -> zbus::Result<i32> {
    let props = zbus::fdo::PropertiesProxy::builder(a11y)
        .destination(service.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;
    let v = props
        .get("org.a11y.atspi.Accessible".try_into()?, "ChildCount")
        .await?;
    i32::try_from(v).map_err(zbus::Error::Variant)
}

/// `org.a11y.atspi.Action` — invoked from the click subcommand to
/// activate a menu item.
#[proxy(interface = "org.a11y.atspi.Action")]
trait Action {
    fn do_action(&self, idx: i32) -> zbus::Result<bool>;

    #[zbus(property, name = "NActions")]
    fn n_actions(&self) -> zbus::Result<i32>;
}

/// Serialised menu item — same JSON shape as v0.2's `dbusmenu::MenuItem`
/// so the QML widget needs zero changes when bridge swaps backends.
///
/// `service` + `path` are the AT-SPI coordinates of THIS item — the
/// QML widget passes them back to the click subcommand
/// (`noctalia-appmenu-bridge atspi-click <service> <path>`) which
/// calls `Action.DoAction(0)` against the right object.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MenuItem {
    /// Stable id within the parent's child list. Currently the
    /// child index; we keep the field for v0.2 wire compat.
    pub id: i32,
    pub label: String,
    /// `"standard"` / `"separator"` / `"submenu"`. Derived from
    /// AT-SPI role + child count.
    #[serde(rename = "type")]
    pub item_type: String,
    pub enabled: bool,
    pub visible: bool,
    /// Always empty in v0.3.0 — AT-SPI doesn't expose icons. v0.3.x
    /// could correlate with `QtIcon` names from action introspection.
    #[serde(default)]
    pub icon_name: String,
    pub toggle_type: String,
    pub toggle_state: i32,
    /// AT-SPI bus name — e.g. `:1.84` (unique connection).
    pub service: String,
    /// AT-SPI object path — e.g. `/org/a11y/atspi/accessible/12`.
    pub path: String,
    #[serde(default)]
    pub children: Vec<MenuItem>,
}

/// Flip `org.a11y.Status.IsEnabled = true` on the session bus.
///
/// AT-SPI defaults this to `false`; Qt's accessibility bridge polls
/// the property at `QApplication` construction and only registers
/// the app's accessible tree on the a11y bus when it's `true`.
/// Without this flip, every Qt app on the system stays invisible to
/// the a11y registry — `Registry.GetChildren()` returns an empty
/// list — which kills our menubar walker.
///
/// On stock GNOME this gets flipped by Orca / dconf. On niri (no
/// AT installed by default) nothing flips it, so the bridge owns
/// it. Setting it once at bridge startup is enough — at-spi2-core
/// keeps it alive until the registry exits.
pub async fn enable_a11y() -> Result<()> {
    let session = Connection::session()
        .await
        .context("connecting to session bus to enable a11y")?;
    let props = zbus::fdo::PropertiesProxy::builder(&session)
        .destination("org.a11y.Bus")?
        .path("/org/a11y/bus")?
        .build()
        .await
        .context("building org.a11y.Bus properties proxy")?;
    props
        .set(
            "org.a11y.Status".try_into()?,
            "IsEnabled",
            &zbus::zvariant::Value::from(true),
        )
        .await
        .context("setting org.a11y.Status.IsEnabled = true")?;
    Ok(())
}

/// Connect to the AT-SPI bus. Returns a `Connection` ready for
/// AT-SPI proxy construction. The session bus is queried first to
/// discover the a11y bus address, then a fresh connection opens
/// against that address.
///
/// Prefer [`AtspiClient::connection`] in the daemon's hot loop —
/// this free function builds a fresh connection per call, which is
/// fine for one-shot CLI subcommands but wasteful for the
/// per-focus-event AT-SPI walk (FR-006).
pub async fn connect_a11y() -> Result<Connection> {
    let session = Connection::session()
        .await
        .context("connecting to session bus to discover a11y address")?;

    let bus_proxy = zbus::Proxy::new(&session, "org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus")
        .await
        .context("building org.a11y.Bus proxy")?;

    let address: String = bus_proxy
        .call("GetAddress", &())
        .await
        .context("calling org.a11y.Bus.GetAddress — is at-spi2-core running?")?;

    zbus::connection::Builder::address(address.as_str())
        .with_context(|| format!("parsing a11y bus address {address}"))?
        .build()
        .await
        .with_context(|| format!("connecting to a11y bus at {address}"))
}

/// Long-lived AT-SPI connection holder (FR-006).
///
/// The daemon's per-focus-event walks share one `zbus::Connection`
/// instead of opening a fresh socket each time (zbus's `Connection`
/// is `Arc`-cloneable; the inner socket survives across all clones).
/// On a11y bus restart the cache is invalidated by [`AtspiClient::invalidate`]
/// (called from the IsEnabled watcher — see [`watch_a11y_status`])
/// so the next [`AtspiClient::connection`] call re-discovers the new
/// bus address.
///
/// The struct is cheap to clone (one `Arc` bump). `main.rs` constructs
/// one instance, hands `.clone()` copies to both the active-snapshot
/// loop and the IsEnabled watcher task.
#[derive(Clone, Default)]
pub struct AtspiClient {
    inner: Arc<Mutex<Option<Connection>>>,
}

impl AtspiClient {
    /// Construct an empty client. The first [`AtspiClient::connection`]
    /// call lazily opens the AT-SPI bus connection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a handle to the cached `zbus::Connection` — opening a
    /// fresh one if the cache is empty (post-startup or post-
    /// [`AtspiClient::invalidate`]).
    pub async fn connection(&self) -> Result<Connection> {
        let mut guard = self.inner.lock().await;
        if let Some(c) = guard.as_ref() {
            return Ok(c.clone());
        }
        let c = connect_a11y().await?;
        *guard = Some(c.clone());
        Ok(c)
    }

    /// Drop the cached connection. The next [`AtspiClient::connection`]
    /// call re-runs `org.a11y.Bus.GetAddress` and opens a fresh socket.
    /// Used by [`watch_a11y_status`] on a11y bus restart.
    pub async fn invalidate(&self) {
        let mut guard = self.inner.lock().await;
        *guard = None;
    }
}

/// Find the AT-SPI Application root for a given PID by walking the
/// registry's children and resolving each app's bus connection's
/// PID via D-Bus.
///
/// Returns `(service, path)` of the matching application, or `None`
/// if no a11y app for this PID is registered. Common reasons for
/// `None`: the app doesn't have AT-SPI integration loaded (Qt apps
/// without `QT_ACCESSIBILITY=1`), or it's a niche toolkit (Electron
/// without `--force-accessibility`).
pub async fn find_app_for_pid(
    a11y: &Connection,
    pid: u32,
    app_id_hint: Option<&str>,
    focus_title: Option<&str>,
) -> Result<Option<(String, OwnedObjectPath)>> {
    let registry = AccessibleProxy::builder(a11y)
        .destination("org.a11y.atspi.Registry")?
        .path("/org/a11y/atspi/accessible/root")?
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await
        .context("building registry root accessible proxy")?;

    let registry_path: ObjectPath<'_> = "/org/a11y/atspi/accessible/root".try_into()?;
    let count = read_child_count(a11y, "org.a11y.atspi.Registry", &registry_path).await?;
    let dbus = zbus::fdo::DBusProxy::new(a11y).await?;

    // Pass 1: PID match. Native Wayland clients show up here directly —
    // niri's wl_client PID equals the AT-SPI app's PID. Fast path; one
    // round-trip per registered app.
    let mut candidates: Vec<(String, OwnedObjectPath)> = Vec::new();
    for i in 0..count {
        let (service, path) = match registry.get_child_at_index(i).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let bus_name: zbus::names::BusName<'_> = match service.as_str().try_into() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let app_pid = match dbus.get_connection_unix_process_id(bus_name).await {
            Ok(p) => p,
            Err(_) => continue,
        };
        if app_pid == pid {
            // ADR-0030: do NOT return the app root for a multi-window
            // process — `find_menubar`'s DFS would grab an arbitrary
            // window's menubar. Scope to the niri-focused window's frame.
            return Ok(resolve_focused_frame(a11y, service, path, focus_title).await);
        }
        candidates.push((service, path));
    }

    // Pass 2: AT-SPI's own active-window detection.
    //
    // The PID join breaks for any window whose wl_client PID differs
    // from the app process's PID:
    //
    // - **XWayland** apps surface as `xwayland-satellite`'s PID to
    //   niri while the app's a11y connection has the X11 client PID.
    // - **Flatpak / bwrap** wrap the app in a sandbox; niri sees the
    //   wrapper PID, AT-SPI sees the namespace-internal PID.
    // - **Subprocess apps** (Anki's wrapper script, web-app shells)
    //   spawn the actual UI process under a launcher PID.
    //
    // Universal fix: ask AT-SPI which application currently holds
    // window-manager focus AND corroborate against `app_id_hint`. Hint
    // gating is critical: AT-SPI's STATE_ACTIVE reflects whichever
    // top-level a11y-aware window the WM most-recently activated, which
    // is NOT necessarily the niri-focused window. Common case: niri
    // focuses a terminal (ghostty has no AT-SPI registration), but
    // STATE_ACTIVE remains set on the previously-focused app (Firefox).
    // Without the hint check we'd return Firefox's menubar for a
    // ghostty-focused frame — a real misrender Pedro caught in v0.3.0
    // -alpha.5 before this gate.
    //
    // The gate: only trust STATE_ACTIVE when the active app's
    // accessible Name fuzzy-matches `app_id_hint`. For native-Wayland
    // apps PID match (pass 1) handles the case directly; the
    // STATE_ACTIVE pass exists for XWayland/sandbox/subprocess paths
    // where PID misses but app_id is still meaningful.
    if app_id_hint.is_some() {
        if let Some(found) = find_active_app_via_state(a11y, &candidates, app_id_hint).await {
            tracing::debug!(
                pid,
                "atspi app matched via STATE_ACTIVE + name corroboration"
            );
            return Ok(Some(found));
        }
    }

    // Pass 3: name-match fallback without STATE_ACTIVE corroboration.
    // For apps whose toolkit doesn't set STATE_ACTIVE consistently
    // (some Electron wrappers, niche toolkits) but whose accessible
    // Name still matches the hint. Lower confidence than pass 2 since
    // we can't confirm the AT-SPI app actually has focus.
    if let Some(hint) = app_id_hint {
        let normalized_hint = normalize_app_id(hint);
        if !normalized_hint.is_empty() {
            for (service, path) in candidates {
                let name = match read_name(a11y, &service, &path.as_ref()).await {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let normalized_name = normalize_app_id(&name);
                if fuzzy_app_match(&normalized_hint, &normalized_name) {
                    tracing::debug!(
                        pid,
                        %hint,
                        %name,
                        "atspi app matched by name fallback (last resort)"
                    );
                    return Ok(Some((service, path)));
                }
            }
        }
    }
    Ok(None)
}

/// The frame-selection verdict for a PID-matched application. Kept as a
/// separate enum so the policy ([`choose_frame`]) is pure and unit-tested
/// without standing up an AT-SPI bus.
#[derive(Debug, PartialEq, Eq)]
enum FrameChoice {
    /// 0 or 1 window — the app root is unambiguous; preserve the proven
    /// single-window path (`find_menubar` DFS finds the lone menubar).
    AppRoot,
    /// The frame path identifying the niri-focused window.
    Frame(OwnedObjectPath),
    /// Multi-window and we cannot tell which is focused — serve the
    /// placeholder rather than an arbitrary (likely wrong) window's menu.
    NoMenu,
}

/// Pure frame-selection policy (ADR-0030). `n_children` is the number of
/// top-level windows under the app root; `title_hit`/`active_hit` are the
/// frame paths matched by the niri window title and by `STATE_ACTIVE`.
///
/// Priority: single-window → app root; else exact title match (the only
/// deterministic same-PID-multi-window discriminator AT-SPI exposes —
/// `STATE_ACTIVE` is not authoritative here, at-spi2-core
/// `constants.h`); else `STATE_ACTIVE` as a tiebreaker; else no menu.
fn choose_frame(
    n_children: usize,
    title_hit: Option<OwnedObjectPath>,
    active_hit: Option<OwnedObjectPath>,
) -> FrameChoice {
    if n_children <= 1 {
        return FrameChoice::AppRoot;
    }
    if let Some(p) = title_hit {
        return FrameChoice::Frame(p);
    }
    if let Some(p) = active_hit {
        return FrameChoice::Frame(p);
    }
    FrameChoice::NoMenu
}

/// Whether an AT-SPI frame's accessible Name identifies the niri-focused
/// window. niri's window title and the toolkit's frame Name are the same
/// string for GTK/Qt/Firefox windows, so an **exact trimmed** match is the
/// safe discriminator: a containment/fuzzy match would mispair Firefox
/// windows that share a long common suffix (" — Firefox Nightly"). On no
/// exact match the caller falls back to `STATE_ACTIVE`, then to no-menu.
fn title_matches(frame_name: &str, focus_title: &str) -> bool {
    let a = frame_name.trim();
    let b = focus_title.trim();
    !a.is_empty() && a == b
}

/// On a PID match we hold the AT-SPI application root. A single process
/// may own several top-level windows (Firefox, okular, LibreOffice) whose
/// menubars differ; `find_menubar`'s first-hit DFS from the app root would
/// grab an arbitrary window's menubar — the wrong-window-routing bug
/// (ADR-0030, drift-trigger-I). Scope the walk to the niri-focused
/// window's frame via [`choose_frame`].
async fn resolve_focused_frame(
    a11y: &Connection,
    service: String,
    app_root: OwnedObjectPath,
    focus_title: Option<&str>,
) -> Option<(String, OwnedObjectPath)> {
    const ACTIVE_BIT: u64 = 1 << 1;

    // Direct children of the app root are its top-level windows
    // (Firefox/Anki/GIMP at depth 1). read_name gives each frame's title.
    let frames = list_frames(a11y, &service, &app_root.as_ref()).await;

    let title_hit = focus_title.and_then(|t| {
        frames
            .iter()
            .find(|(_, name)| title_matches(name, t))
            .map(|(p, _)| p.clone())
    });
    // STATE_ACTIVE tiebreaker only when title-matching was inconclusive.
    // Depth 2 covers Qt's `Application → AppName → frame` nesting.
    let active_hit = if frames.len() > 1 && title_hit.is_none() {
        scan_for_active_frame(a11y, &service, &app_root.as_ref(), 2, ACTIVE_BIT).await
    } else {
        None
    };

    match choose_frame(frames.len(), title_hit, active_hit) {
        FrameChoice::AppRoot => Some((service, app_root)),
        FrameChoice::Frame(path) => {
            tracing::debug!(%service, frame = %path, "atspi frame-scoped to focused window");
            Some((service, path))
        }
        FrameChoice::NoMenu => {
            tracing::debug!(
                %service,
                frames = frames.len(),
                "multi-window app: no frame matched focus title or STATE_ACTIVE — serving placeholder"
            );
            None
        }
    }
}

/// Enumerate an application root's direct children (its top-level windows)
/// as `(frame_path, accessible_name)`. The Name is the window title used
/// by [`resolve_focused_frame`] to pick the niri-focused window. Errors on
/// individual children are skipped (best-effort), mirroring the rest of
/// the walker.
async fn list_frames(
    a11y: &Connection,
    service: &str,
    app_root: &ObjectPath<'_>,
) -> Vec<(OwnedObjectPath, String)> {
    let mut frames = Vec::new();
    let proxy = match AccessibleProxy::builder(a11y)
        .destination(service.to_owned())
        .and_then(|b| b.path(app_root.to_owned()))
        .map(|b| b.cache_properties(zbus::CacheProperties::No))
    {
        Ok(b) => match b.build().await {
            Ok(p) => p,
            Err(_) => return frames,
        },
        Err(_) => return frames,
    };
    let count = read_child_count(a11y, service, app_root).await.unwrap_or(0);
    for i in 0..count {
        let (child_service, child_path) = match proxy.get_child_at_index(i).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Crossing bus connections under one app is uncommon; the foreign
        // proxy would just fail. Keep to the same service.
        if child_service != service {
            continue;
        }
        let name = read_name(a11y, service, &child_path.as_ref())
            .await
            .unwrap_or_default();
        frames.push((child_path, name));
    }
    frames
}

/// Walk each candidate AT-SPI app's tree (depth ≤ 2) looking for the
/// **frame** with `STATE_ACTIVE` (bit 1) set. Returns
/// `(service, frame_path)` — the path to the active frame, NOT the app
/// root.
///
/// **Why frame-scoped, not app-scoped (codex review of PR #38):** apps
/// with multiple top-level windows (okular with 3 PDFs open,
/// LibreOffice with both Writer + Calc, multi-profile Firefox) expose
/// multiple frame children under one Application. If we returned the
/// app root, `find_menubar`'s DFS would pick the FIRST `MENU_BAR` it
/// encounters — which may belong to an unfocused window. Returning
/// the active frame path scopes the menubar walk to the correct
/// window.
///
/// **Tie-breaking on race (codex review):** AT-SPI state propagation
/// is async; during alt-tab transitions, two apps may briefly both
/// have STATE_ACTIVE set. When `app_id_hint` is provided, prefer
/// candidates whose accessible Name fuzzy-matches it. Without a hint,
/// first-found wins.
///
/// **Bit semantics:** `AtspiStateType::ATSPI_STATE_ACTIVE = 1` →
/// bit-index 1 → mask `1 << 1 = 0b10`. State wire format is `au`
/// (array of two u32); we OR them into a 64-bit view.
///
/// Depth 2 covers every app structure observed in the wild:
/// - Depth 1: Firefox / Anki / GIMP (frames directly under root).
/// - Depth 2: some Qt apps wrap frames in `Application → AppName →
///   frame`.
async fn find_active_app_via_state(
    a11y: &Connection,
    candidates: &[(String, OwnedObjectPath)],
    app_id_hint: Option<&str>,
) -> Option<(String, OwnedObjectPath)> {
    const ACTIVE_BIT: u64 = 1 << 1;

    let Some(normalized_hint) = app_id_hint.map(normalize_app_id).filter(|s| !s.is_empty()) else {
        // No hint → can't corroborate. Caller should not invoke us
        // without a hint; return None defensively.
        return None;
    };

    for (service, app_path) in candidates {
        let Some(frame_path) =
            scan_for_active_frame(a11y, service, &app_path.as_ref(), 2, ACTIVE_BIT).await
        else {
            continue;
        };
        // REQUIRE name corroboration. AT-SPI's STATE_ACTIVE survives
        // niri focus changes when the niri-focused window has no AT-SPI
        // representation (terminals, electron-no-a11y, etc), which
        // would otherwise cause the bridge to render the previously-
        // focused app's menubar over an unrelated window.
        let Ok(name) = read_name(a11y, service, &app_path.as_ref()).await else {
            continue;
        };
        let n = normalize_app_id(&name);
        if fuzzy_app_match(&normalized_hint, &n) {
            return Some((service.clone(), frame_path));
        }
    }
    None
}

/// Recursive helper for `find_active_app_via_state`. Returns the path
/// of the first descendant within `cur_depth` levels whose state has
/// `bit` set, or `None`. Returns the **descendant's** path, not the
/// caller's, so the result identifies the actual active frame rather
/// than its enclosing application.
async fn scan_for_active_frame(
    a11y: &Connection,
    service: &str,
    path: &ObjectPath<'_>,
    cur_depth: u32,
    bit: u64,
) -> Option<OwnedObjectPath> {
    let proxy = match AccessibleProxy::builder(a11y)
        .destination(service.to_owned())
        .and_then(|b| b.path(path.to_owned()))
        .map(|b| b.cache_properties(zbus::CacheProperties::No))
    {
        Ok(b) => match b.build().await {
            Ok(p) => p,
            Err(_) => return None,
        },
        Err(_) => return None,
    };

    if let Ok(states) = proxy.get_state().await {
        let lo = u64::from(*states.first().unwrap_or(&0));
        let hi = u64::from(*states.get(1).unwrap_or(&0));
        let combined = (hi << 32) | lo;
        if combined & bit != 0 {
            return Some(path.to_owned().into());
        }
    }

    if cur_depth == 0 {
        return None;
    }
    let count = read_child_count(a11y, service, path).await.unwrap_or(0);
    for i in 0..count {
        let (child_service, child_path) = match proxy.get_child_at_index(i).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Only descend into children of the same service — crossing
        // bus connections under one app is uncommon and the foreign
        // proxy build would just fail.
        if child_service != service {
            continue;
        }
        if let Some(found) = Box::pin(scan_for_active_frame(
            a11y,
            &child_service,
            &child_path.as_ref(),
            cur_depth - 1,
            bit,
        ))
        .await
        {
            return Some(found);
        }
    }
    None
}

/// Normalize a Wayland app-id or AT-SPI accessible Name for fuzzy
/// equality. Strips reverse-DNS prefixes (`org.kde.`, `com.mitchellh.`),
/// lowercases, and trims whitespace. Examples:
///
/// - `Anki` → `anki`
/// - `org.kde.okular` → `okular`
/// - `firefox-nightly` → `firefox-nightly`
fn normalize_app_id(s: &str) -> String {
    let trimmed = s.trim().to_lowercase();
    if let Some(idx) = trimmed.rfind('.') {
        // strip reverse-DNS prefix iff the prefix has 2+ dots
        // (`org.kde.x` strips to `x`; `firefox-nightly` keeps as-is).
        let prefix = &trimmed[..idx];
        if prefix.contains('.') {
            return trimmed[idx + 1..].to_string();
        }
    }
    trimmed
}

/// FR-008 pass-2 fuzzy match. Two normalized identifiers are
/// considered the same app when the shorter is a substring of the
/// longer AND the shorter is at least 3 characters long. Used by
/// both `find_app_for_pid`'s name-fallback branch and
/// `find_active_app_via_state`'s corroboration step.
///
/// The 3-char floor avoids accidental matches on common 2-letter
/// fragments (`qt`, `ui`, `vm`) that would otherwise mispair
/// unrelated apps. Real-world coverage:
///
/// - `kate` vs `kate` (KDE double-prefix → `normalize_app_id`
///   strips `org.kde.` on both sides, fuzzy returns true even on
///   exact equality).
/// - `anki` (niri-reported wrapper `app_id`) vs `anki.bin` (AT-SPI
///   `Name` from the actual Qt subprocess) → `anki` is a substring
///   of `anki.bin`, match succeeds.
/// - `ok` vs `okular` → short side is 2 chars, no match (safety).
fn fuzzy_app_match(normalized_hint: &str, normalized_name: &str) -> bool {
    if normalized_hint.is_empty() || normalized_name.is_empty() {
        return false;
    }
    let (short, long) = if normalized_name.len() < normalized_hint.len() {
        (normalized_name, normalized_hint)
    } else {
        (normalized_hint, normalized_name)
    };
    short.len() >= 3 && long.contains(short)
}

/// Depth-first search for the first descendant with role
/// `MENU_BAR` under the given accessible.
///
/// `cur_depth` is the recursion guard. Apps that bury their
/// `MenuBar` more than `MAX_FIND_DEPTH` levels deep simply return
/// `None` — we'd rather miss a pathological app than hang on a
/// runaway walk.
pub async fn find_menubar(
    a11y: &Connection,
    service: &str,
    path: &ObjectPath<'_>,
    cur_depth: u32,
) -> Result<Option<(String, OwnedObjectPath)>> {
    if cur_depth >= MAX_FIND_DEPTH {
        return Ok(None);
    }
    let proxy = AccessibleProxy::builder(a11y)
        .destination(service.to_owned())?
        .path(path.to_owned())?
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await
        .context("building accessible proxy for find")?;

    if proxy.get_role().await.unwrap_or(0) == role::MENU_BAR {
        return Ok(Some((service.to_owned(), path.to_owned().into())));
    }

    let count = read_child_count(a11y, service, path).await.unwrap_or(0);
    for i in 0..count {
        let (child_service, child_path) = match proxy.get_child_at_index(i).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(found) = Box::pin(find_menubar(
            a11y,
            &child_service,
            &child_path.as_ref(),
            cur_depth + 1,
        ))
        .await?
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

/// Walk a `MenuBar`'s subtree into a serializable `MenuItem` tree.
///
/// `cur_depth` bounds recursion. Items past `MAX_FETCH_DEPTH` get
/// truncated to `children: []` — same as `DBusMenu`'s lazy-load
/// behavior, the QML widget can request a deeper fetch on hover
/// once we wire that (v0.3.x).
pub async fn fetch_menu_tree(
    a11y: &Connection,
    service: &str,
    path: &ObjectPath<'_>,
    cur_depth: u32,
) -> Result<MenuItem> {
    let proxy = AccessibleProxy::builder(a11y)
        .destination(service.to_owned())?
        .path(path.to_owned())?
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await
        .context("building accessible proxy for fetch")?;

    let label = read_name(a11y, service, path).await.unwrap_or_default();
    let role = proxy.get_role().await.unwrap_or(0);
    let states = proxy.get_state().await.unwrap_or_default();
    let count = read_child_count(a11y, service, path).await.unwrap_or(0);

    // AT-SPI state bitmask spans two u32 words. Indices come from
    // atspi-constants.h `AtspiStateType` (verified live 2026-05-06):
    //   ENABLED   = 8   (bit 8 of word 0)
    //   SENSITIVE = 24  (bit 24 of word 0) — needed for interactive
    //   VISIBLE   = 31  (bit 31 of word 0)
    //   SHOWING   = 25  (bit 25 of word 0) — currently rendered
    //
    // For widget UX: `enabled` is "user can click" → ENABLED &&
    // SENSITIVE. `visible` for the QML widget means "include in the
    // tree" — closed-submenu items are SHOWING=0 but should still
    // render in the list, so we treat any tree-present item as
    // visible. Apps that explicitly set VISIBLE=0 are surfaced via
    // the bit so the QML widget can hide them if it wants.
    let lo = u64::from(*states.first().unwrap_or(&0));
    let hi = u64::from(*states.get(1).unwrap_or(&0));
    let state = (hi << 32) | lo;
    let enabled = (state & (1u64 << 8)) != 0 && (state & (1u64 << 24)) != 0;
    // Most menu items have VISIBLE=0 unless their parent submenu is
    // currently open — gating QML render on this would hide every
    // item until hover. We always include items that exist in the
    // tree; the parent's open/closed state is the QML widget's
    // concern. (DEFUNCT items were already filtered upstream — Qt
    // doesn't expose them as accessibles.)
    let visible = true;

    // Qt represents top-level menubar items as MENU_ITEM (role 35)
    // with one MENU (role 33) child holding the popup. So "has
    // children" is the canonical submenu predicate — role alone
    // mislabels Qt's menubar children as standard items.
    let item_type = if role == role::SEPARATOR {
        "separator".to_string()
    } else if count > 0 || role == role::MENU || role == role::MENU_BAR {
        "submenu".to_string()
    } else {
        "standard".to_string()
    };

    let toggle_type = match role {
        role::CHECK_MENU_ITEM => "checkmark".to_string(),
        role::RADIO_MENU_ITEM => "radio".to_string(),
        _ => String::new(),
    };

    // AT-SPI exposes toggle state via CHECKED (bit 4 in
    // `AtspiStateType` — `ATSPI_STATE_CHECKED = 4`).
    let toggle_state = i32::from(!toggle_type.is_empty() && (state & (1u64 << 4)) != 0);

    let mut item = MenuItem {
        id: 0,
        label,
        item_type,
        enabled,
        visible,
        icon_name: String::new(),
        toggle_type,
        toggle_state,
        service: service.to_string(),
        path: path.to_string(),
        children: Vec::new(),
    };

    if cur_depth >= MAX_FETCH_DEPTH {
        return Ok(item);
    }

    // v1.0.8 — bounded-parallel children walk.
    //
    // Pedro field report 16/05/2026 post-v1.0.7: Firefox menu clicks
    // on Bookmarks/Profiles/Tools/Help opened blank popups because the
    // (then) 500ms `FETCH_BUDGET` cut the sequential walk before the
    // back half of the top-level items were descended into. With this
    // crate's strict per-PID cache the partial tree then served the
    // next 30s of clicks. Two changes broke the deadlock:
    //   1. `FETCH_BUDGET` raised to 2500ms (defence in depth — covers
    //      pathological cases). See `fetch_menubar_for_pid`.
    //   2. Children walked with `buffered(8)` — up to 8 in-flight
    //      D-Bus round-trips per tree level, joined in tree-index
    //      order. Concurrency is bounded so a malformed AT-SPI app
    //      can't fan out to thousands of connections.
    //
    // Stage 1: resolve the (service, path) tuple for every child up
    // front. `proxy.get_child_at_index` is a cheap registry lookup
    // (~5 ms × N children) and we'd otherwise need to hand each
    // closure a clone of the parent proxy — easier to materialise
    // the refs here.
    let mut child_refs: Vec<(i32, String, OwnedObjectPath)> = Vec::with_capacity(count as usize);
    for i in 0..count {
        if let Ok((cs, cp)) = proxy.get_child_at_index(i).await {
            child_refs.push((i, cs, cp));
        }
    }

    // Stage 2: parallel-walk each subtree. `buffered(8)` caps in-flight
    // work (matches AT-SPI's per-connection throughput) and preserves
    // index order so `child.id = i` stays correct.
    let children: Vec<MenuItem> = stream::iter(child_refs)
        .map(|(i, cs, cp)| async move {
            let mut child = Box::pin(fetch_menu_tree(a11y, &cs, &cp.as_ref(), cur_depth + 1))
                .await
                .ok()?;
            child.id = i;
            Some(child)
        })
        .buffered(8)
        .filter_map(|opt| async move { opt })
        .collect()
        .await;
    item.children = children;

    // Spec 009 FR-001 — recursive Qt wrapper-flatten.
    flatten_qt_wrapper(&mut item);

    Ok(item)
}

/// Strip Qt's unnamed-MENU popup wrapper from a single `MenuItem`.
///
/// Qt wraps every `MENU_ITEM`'s popup in an unnamed `MENU` child:
///
/// ```text
///   File (MENU_ITEM, label "File", 1 child)
///     └── "" (MENU, label "", N children) ← popup wrapper
///           ├── Open (MENU_ITEM)
///           └── Save (MENU_ITEM)
/// ```
///
/// QML renders `item.children` directly under each menu button —
/// without flattening, the popup would show a single empty-label
/// entry and the real items would be hidden one level deeper.
///
/// **Recursive correctness (spec 009 FR-001).** `fetch_menu_tree`
/// invokes this helper at the END of every recursive call, so by
/// induction on depth every returned tree is wrapper-stripped at
/// every level. Extracting the in-line check into a named helper
/// makes the invariant explicit and unit-testable without an
/// AT-SPI bus (see `bridge/tests/atspi_flatten.rs`).
///
/// **Idempotent.** Safe to invoke multiple times on the same item;
/// once flattened, the predicate no longer matches.
pub(crate) fn flatten_qt_wrapper(item: &mut MenuItem) {
    if item.children.len() == 1
        && item.children[0].label.is_empty()
        && !item.children[0].children.is_empty()
    {
        let mut grandchildren = std::mem::take(&mut item.children[0].children);
        for (i, c) in grandchildren.iter_mut().enumerate() {
            c.id = i as i32;
        }
        item.children = grandchildren;
    }
}

/// One-shot helper: from a focused PID, return the parsed menu
/// tree (or `None` when no menubar found).
///
/// Composes [`AtspiClient::connection`] → `find_app_for_pid` →
/// `find_menubar` → `fetch_menu_tree`. Each step's failure surfaces
/// as `Err(_)` so the caller can log a warn and write a `null` menu
/// to active.json — letting the QML widget fall back to its
/// placeholder gracefully.
///
/// **Total budget:** the whole pipeline is wrapped in a 3 s
/// `tokio::time::timeout`. AT-SPI calls cross the focused app's
/// process boundary; a hung or slow app must not freeze the bridge
/// (codex P1 #3). On timeout we return `Ok(None)` so the QML widget
/// falls back to the v0.1 placeholder rather than holding the bar
/// in a stale state.
///
/// **Persistent connection (FR-006):** the AT-SPI socket is owned
/// by `client` and reused across focus events. On bus restart the
/// [`watch_a11y_status`] task invalidates the cache and the next
/// call here re-discovers the new bus address.
///
/// **GTK4 empty fallback (FR-004):** when the post-walk tree has
/// zero children we return `Ok(None)` instead of an empty
/// `MenuItem`. `active.rs` substitutes the synthetic pseudo-menu so
/// the bar always renders something useful.
pub async fn fetch_menubar_for_pid(
    client: &AtspiClient,
    pid: u32,
    app_id_hint: Option<&str>,
    focus_title: Option<&str>,
) -> Result<Option<MenuItem>> {
    // v1.0.8 — raised 500 ms → 2500 ms now that the children walk
    // is parallel and the X11 case is fully covered by the skip-list.
    //
    // History:
    //   - pre-v1.0.2: 3000 ms (Qt+GTK headroom; X11 apps via
    //     xwayland-satellite blew the whole window on every focus).
    //   - v1.0.2: 500 ms (defended against the X11 case at the cost
    //     of cutting Firefox walks mid-tree).
    //   - v1.0.6: skip-list added — xwayland-satellite + terminals
    //     no longer reach this code path at all.
    //   - v1.0.8: 2500 ms (covers Pedro's 500ms-2.1s Firefox walks
    //     end-to-end; parallel walk usually finishes in <500 ms
    //     anyway so this is defence in depth).
    //
    // Real AT-SPI-capable apps (Qt6/GTK4) typically respond in <50 ms;
    // hitting the 2500 ms timeout means the app is genuinely hung and
    // we want the placeholder fall-back, not a frozen bar.
    const FETCH_BUDGET: std::time::Duration = std::time::Duration::from_millis(2500);
    match tokio::time::timeout(
        FETCH_BUDGET,
        fetch_menubar_for_pid_inner(client, pid, app_id_hint, focus_title),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!(
                pid,
                budget_ms = FETCH_BUDGET.as_millis(),
                "atspi fetch timed out — focused app slow/hung; widget falls back to placeholder"
            );
            Ok(None)
        }
    }
}

async fn fetch_menubar_for_pid_inner(
    client: &AtspiClient,
    pid: u32,
    app_id_hint: Option<&str>,
    focus_title: Option<&str>,
) -> Result<Option<MenuItem>> {
    let a11y = client.connection().await?;
    let app = match find_app_for_pid(&a11y, pid, app_id_hint, focus_title).await? {
        Some(a) => a,
        None => return Ok(None),
    };
    let menubar = match find_menubar(&a11y, &app.0, &app.1.as_ref(), 0).await? {
        Some(mb) => mb,
        None => return Ok(None),
    };
    let tree = fetch_menu_tree(&a11y, &menubar.0, &menubar.1.as_ref(), 0).await?;
    if menubar_is_empty(&tree) {
        tracing::debug!(
            pid,
            "menubar walk returned zero children — caller should fall back to synthetic menu"
        );
        return Ok(None);
    }
    Ok(Some(tree))
}

/// FR-004 predicate: a freshly-walked menubar accessible with zero
/// children is the GTK4 `GtkPopoverMenuBar` quirk (Nautilus 45+ and
/// other GTK4 apps that defer child realisation until popup). Other
/// app classes also surface empty menubars (Qt apps mid-tear-down,
/// half-initialised electron wrappers). Routing the empty case
/// through `Ok(None)` lets the caller substitute the synthetic
/// pseudo-menu instead of rendering a blank bar.
fn menubar_is_empty(tree: &MenuItem) -> bool {
    tree.children.is_empty()
}

/// Polls `org.a11y.Status.IsEnabled` on the session bus and re-flips
/// it to `true` whenever it goes false (FR-005). Also invalidates
/// `client`'s cached connection so the next walk re-discovers the
/// (potentially restarted) a11y bus address.
///
/// **Why polling, not signal subscription:** the
/// `org.freedesktop.DBus.Properties.PropertiesChanged` route needs
/// a `futures_util::StreamExt` (or equivalent) bring-in. The
/// observable latency budget per Scenario 5 (≤ 5 s) is well within
/// the 3 s polling interval, and the extra dep wasn't justified
/// for this single use site. See plan.md §Open questions.
///
/// Loops forever — call inside a long-lived task. Per-iteration
/// errors are logged + caching is invalidated; the loop never exits
/// so a transient a11y bus restart self-heals on the next poll.
pub async fn watch_a11y_status(client: AtspiClient) -> Result<()> {
    const POLL: std::time::Duration = std::time::Duration::from_secs(3);
    loop {
        tokio::time::sleep(POLL).await;
        match is_a11y_enabled().await {
            Ok(true) => {}
            Ok(false) => {
                tracing::info!(
                    "a11y IsEnabled observed false; re-enabling, invalidating AT-SPI cache, \
                     clearing expensive learned-skips (off-bus apps may re-register on restart)"
                );
                client.invalidate().await;
                clear_expensive_skips();
                if let Err(e) = enable_a11y().await {
                    tracing::warn!(error = ?e, "re-enable a11y after IsEnabled flip failed");
                }
            }
            Err(e) => {
                tracing::debug!(
                    error = ?e,
                    "a11y IsEnabled probe failed (bus restart in flight?); invalidating cache \
                     + clearing expensive learned-skips (apps re-register after the restart)"
                );
                client.invalidate().await;
                // A probe failure is the actual a11y-bus-restart signal
                // (an IsEnabled poll that errors, not one that returns
                // false). Clear here too, else an off-bus app that
                // re-registers on restart stays skipped until the
                // EXPENSIVE_RECHECK_TTL backstop (ADR-0033).
                clear_expensive_skips();
            }
        }
    }
}

/// Read the current value of `org.a11y.Status.IsEnabled` on the
/// session bus. Returns `Err` if `at-spi2-core` is not running.
async fn is_a11y_enabled() -> Result<bool> {
    let session = Connection::session()
        .await
        .context("a11y status probe: connecting to session bus")?;
    let props = zbus::fdo::PropertiesProxy::builder(&session)
        .destination("org.a11y.Bus")?
        .path("/org/a11y/bus")?
        .build()
        .await
        .context("a11y status probe: building PropertiesProxy")?;
    let v = props
        .get("org.a11y.Status".try_into()?, "IsEnabled")
        .await
        .context("a11y status probe: Get(IsEnabled)")?;
    bool::try_from(v).context("a11y status probe: bool conversion")
}

/// Typed error variants surfaced by AT-SPI operations. Generic
/// failures still funnel through [`anyhow::Error`]; only the
/// variants callers need to discriminate on are typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuError {
    /// `do_action` could not resolve the addressed accessible — the
    /// focused app rebuilt its widget tree between when the bridge
    /// walked it and when the click arrived. Carries the originating
    /// (service, path) so the caller can log + signal a snapshot
    /// refresh. Spec 005 FR-007.
    Stale { service: String, path: String },
}

impl std::fmt::Display for MenuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stale { service, path } => write!(
                f,
                "MenuError::Stale {{ service: {service:?}, path: {path:?} }}"
            ),
        }
    }
}

impl std::error::Error for MenuError {}

/// Click subcommand backend: invoke `Action.DoAction(0)` on the
/// AT-SPI accessible at the given (service, path). Action index
/// 0 is "click" by qtatspi convention.
///
/// **Synthetic-menu items (PR #42):** when the focused app has no
/// AT-SPI menubar (terminals, electron-no-a11y) the bridge writes
/// a synthesised "Window" submenu so the bar stays useful. Those
/// items carry `service = "::synthetic"` and `path = "niri:<action>"`
/// (e.g. `niri:close-window`). On click we route through niri-IPC
/// instead of AT-SPI's `DoAction`.
///
/// **Stale-path detection (FR-007):** before issuing `DoAction(0)`
/// the function re-fetches the accessible via a `GetRole` probe.
/// If the app rebuilt its widget tree between snapshot and click,
/// the probe errors and we surface [`MenuError::Stale`] instead of
/// letting the raw `UnknownObject` / `UnknownMethod` D-Bus error
/// bubble up untyped. Callers `downcast_ref::<MenuError>()` to
/// recognise this and trigger an immediate snapshot refresh.
pub async fn do_action(service: &str, path: &str) -> Result<()> {
    if service == SYNTHETIC_SERVICE {
        return dispatch_synthetic(path).await;
    }
    let a11y = connect_a11y().await?;
    let proxy_path: ObjectPath<'_> = path
        .try_into()
        .with_context(|| format!("parsing AT-SPI path {path}"))?;

    // FR-007: re-fetch the addressed accessible. A successful
    // `GetRole` round-trip is sufficient evidence that the path
    // still resolves on the focused app's side. The cheap probe
    // (one method call, no children walked) keeps the click hot
    // path under a millisecond.
    let accessible = AccessibleProxy::builder(&a11y)
        .destination(service.to_owned())?
        .path(proxy_path.clone())?
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await
        .context("re-fetching accessible before DoAction")?;
    if let Err(probe_err) = accessible.get_role().await {
        tracing::debug!(
            service,
            path,
            error = ?probe_err,
            "AT-SPI path stale (re-fetch probe failed); surfacing MenuError::Stale"
        );
        return Err(MenuError::Stale {
            service: service.to_string(),
            path: path.to_string(),
        }
        .into());
    }

    let action = ActionProxy::builder(&a11y)
        .destination(service.to_owned())?
        .path(proxy_path)?
        .build()
        .await
        .context("building Action proxy")?;
    let ok = action
        .do_action(0)
        .await
        .with_context(|| format!("DoAction(0) on {service} {path}"))?;
    if !ok {
        anyhow::bail!("DoAction returned false — item not actionable");
    }
    Ok(())
}

/// Realization settle: how long to wait after `DoAction(0)`-expanding a
/// lazily-built menu before walking its now-realized children. Firefox
/// (and GTK) populate submenu items asynchronously when the menu opens;
/// 150 ms is comfortably above the observed realize latency (~tens of ms,
/// measured live on niri) while keeping the on-click flash brief.
/// ADR-0034.
const EXPAND_REALIZE_DELAY: std::time::Duration = std::time::Duration::from_millis(150);

/// Expand-on-demand for lazily-realized menus (ADR-0034). Firefox and GTK
/// expose a top-level menu (File/Edit/…) over AT-SPI with **zero
/// children** until it is actually opened, so a passive walk sees an
/// empty submenu and the plugin renders nothing ("click does nothing").
/// This fires the menu's `"click"` action to expand it, walks the
/// now-realized subtree, then fires `"click"` again to collapse it.
///
/// Verified live (2026-06-02, Firefox 153 on niri): `File` goes 0 → 17
/// children on expand, and the realized children **and their object
/// paths persist after collapse** — so the returned tree's leaves stay
/// click-safe (a later `do_action` on a leaf resolves), and the expand is
/// effectively one-time per menu.
///
/// Mirrors [`do_action`]'s stale probe: a `GetRole` round-trip first, so a
/// torn-down path surfaces [`MenuError::Stale`] (exit-2 in the subcommand)
/// instead of an untyped D-Bus error.
pub async fn expand_and_fetch(service: &str, path: &str) -> Result<MenuItem> {
    if service == SYNTHETIC_SERVICE {
        anyhow::bail!("synthetic node {path} has no lazy subtree to expand");
    }
    let a11y = connect_a11y().await?;
    let proxy_path: ObjectPath<'_> = path
        .try_into()
        .with_context(|| format!("parsing AT-SPI path {path}"))?;

    let accessible = AccessibleProxy::builder(&a11y)
        .destination(service.to_owned())?
        .path(proxy_path.clone())?
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await
        .context("re-fetching accessible before expand")?;
    if accessible.get_role().await.is_err() {
        return Err(MenuError::Stale {
            service: service.to_string(),
            path: path.to_string(),
        }
        .into());
    }

    let action = ActionProxy::builder(&a11y)
        .destination(service.to_owned())?
        .path(proxy_path.clone())?
        .build()
        .await
        .context("building Action proxy for expand")?;

    // Expand to realize the lazy children. A `false` return just means
    // "not separately actionable" — keep going and walk it anyway (it may
    // already be populated); don't hard-fail.
    let _ = action.do_action(0).await.context("DoAction(0) expand")?;
    tokio::time::sleep(EXPAND_REALIZE_DELAY).await;

    let tree = fetch_menu_tree(&a11y, service, &proxy_path, 0)
        .await
        .context("walking expanded menu subtree")?;

    // Collapse (best-effort). The "click" action toggles, so this closes
    // the menu we just opened; realized children persist regardless
    // (verified), so the returned tree stays click-safe. Only toggle when
    // the walk actually saw children — an empty walk may mean the menu
    // already auto-closed, and a second toggle would re-open it.
    if !tree.children.is_empty() {
        let _ = action.do_action(0).await;
    }

    Ok(tree)
}

/// Sentinel `service` value that the bridge writes for synthetic
/// menu items. The QML widget passes this back unchanged on click;
/// `do_action` recognises it and dispatches to a non-AT-SPI handler.
pub const SYNTHETIC_SERVICE: &str = "::synthetic";

/// Build a macOS-style top-level synthetic menubar so the bar
/// always renders SOMETHING useful, even for apps without AT-SPI
/// menubar (terminals, electron-no-a11y, native Wayland clients
/// with no a11y plugin).
///
/// **Honest-only design (PR #44).** Earlier alpha.10 included an
/// Edit submenu (Cut / Copy / Paste / Select All) that fired
/// keystrokes via `wtype`. Web research (deep-researcher 2026-05-07)
/// confirmed this was a UX trap: the menu lies about what's bound —
/// custom keymaps, vim-mode, US-Intl layouts, or Ghostty config
/// overrides all break the contract silently. **A blank-but-honest
/// menubar beats a lying one.**
///
/// What survives:
///
/// - **Application** (`<App Name>` submenu) — Quit (niri-IPC
///   close-window). One real action.
/// - **Window** — Close / Fullscreen / Floating / Move to ±
///   workspace, all via niri-IPC. Real compositor primitives, not
///   keystroke injection.
///
/// What's gone:
///
/// - **Edit submenu** — was alpha.10's wtype-driven Cut/Copy/Paste.
///   Removed because faked Edit was the largest source of "bar
///   behaves wrong" complaints. Users who need Copy can still use
///   their actual keybinding; the menubar surfaces real commands,
///   not impersonations.
/// - **File / View / Help** — never shipped, no good universal
///   mapping exists.
///
/// Wire form matches `MenuItem` exactly so the QML widget needs zero
/// changes — same render path, same click forwarder. Synthetic items
/// carry sentinel `service = SYNTHETIC_SERVICE` and `path = "niri:<action>"`
/// (or `"niri:noop"` for parent submenus), routed through
/// `dispatch_synthetic` instead of AT-SPI's `Action.DoAction`.
#[must_use]
pub fn synthetic_menu(app_id: &str) -> MenuItem {
    let pretty = pretty_app_label(app_id);
    let app_submenu = synthetic_application_submenu(&pretty);
    let window_submenu = synthetic_window_submenu();
    MenuItem {
        id: 0,
        label: pretty,
        item_type: "submenu".to_string(),
        enabled: true,
        visible: true,
        icon_name: String::new(),
        toggle_type: String::new(),
        toggle_state: 0,
        service: SYNTHETIC_SERVICE.to_string(),
        path: "niri:noop".to_string(),
        children: vec![app_submenu, window_submenu],
    }
}

/// Map a niri action to a freedesktop icon-theme name for the synthetic
/// menu leaves, or `""` when no standard name cleanly fits (the QML
/// widget then renders the row without an icon — never a wrong one).
/// Pure, so it is unit-tested without standing up a menu.
pub(crate) fn niri_action_icon(action: &str) -> &'static str {
    match action {
        "close-window" => "window-close",
        "fullscreen-window" => "view-fullscreen",
        "move-window-to-workspace-down" => "go-down",
        "move-window-to-workspace-up" => "go-up",
        "move-window-to-monitor-left" => "go-previous",
        "move-window-to-monitor-right" => "go-next",
        // No widely-themed standard name for floating / column ops / noop.
        _ => "",
    }
}

/// `niri:<action>` leaf factory. ID is the parent's child index. The
/// `icon_name` is derived from the action via [`niri_action_icon`] so the
/// synthetic Window / Quit leaves theme consistently with the
/// `.desktop`-derived action leaves (which carry the app's own icon).
pub(crate) fn niri_leaf(id: i32, label: &str, action: &str) -> MenuItem {
    MenuItem {
        id,
        label: label.to_string(),
        item_type: "standard".to_string(),
        enabled: true,
        visible: true,
        icon_name: niri_action_icon(action).to_string(),
        toggle_type: String::new(),
        toggle_state: 0,
        service: SYNTHETIC_SERVICE.to_string(),
        path: format!("niri:{action}"),
        children: Vec::new(),
    }
}

/// A non-clickable separator row for grouping synthetic submenu items.
/// Empty `service`/`path` so the QML click guard (`fireClick` requires
/// both) treats it as inert.
pub(crate) fn synthetic_separator(id: i32) -> MenuItem {
    MenuItem {
        id,
        label: String::new(),
        item_type: "separator".to_string(),
        enabled: false,
        visible: true,
        icon_name: String::new(),
        toggle_type: String::new(),
        toggle_state: 0,
        service: String::new(),
        path: String::new(),
        children: Vec::new(),
    }
}

/// Submenu wrapping `children` under `label`. Submenu's own click
/// path is the documented `niri:noop` so accidental leaf-style click
/// (which the QML widget guards against anyway) is a no-op.
pub(crate) fn synthetic_submenu(id: i32, label: &str, children: Vec<MenuItem>) -> MenuItem {
    MenuItem {
        id,
        label: label.to_string(),
        item_type: "submenu".to_string(),
        enabled: true,
        visible: true,
        icon_name: String::new(),
        toggle_type: String::new(),
        toggle_state: 0,
        service: SYNTHETIC_SERVICE.to_string(),
        path: "niri:noop".to_string(),
        children,
    }
}

/// Application submenu — currently just Quit (niri close-window).
/// The submenu's user-facing label is the app's pretty name to
/// match macOS layout (`Ghostty` → `About Ghostty` / `Quit Ghostty`).
fn synthetic_application_submenu(pretty: &str) -> MenuItem {
    synthetic_submenu(
        0,
        pretty,
        vec![niri_leaf(0, &format!("Quit {pretty}"), "close-window")],
    )
}

/// Window submenu — universal niri-IPC actions, grouped macOS-style:
/// window state · column layout · placement. Every action name is
/// verified against `niri msg action` for niri 26.04; an action niri
/// does not recognise is a logged no-op, never a crash. Column/monitor
/// ops are niri-native primitives a daily-driver actually uses, so the
/// fallback Window menu is useful and not just Close.
pub(crate) fn synthetic_window_submenu() -> MenuItem {
    synthetic_submenu(
        1,
        "Window",
        vec![
            niri_leaf(0, "Close", "close-window"),
            niri_leaf(1, "Toggle Fullscreen", "fullscreen-window"),
            niri_leaf(2, "Toggle Floating", "toggle-window-floating"),
            synthetic_separator(3),
            niri_leaf(4, "Maximize Column", "maximize-column"),
            niri_leaf(5, "Center Column", "center-column"),
            niri_leaf(
                6,
                "Expand Column to Available Width",
                "expand-column-to-available-width",
            ),
            synthetic_separator(7),
            niri_leaf(
                8,
                "Move to Previous Workspace",
                "move-window-to-workspace-up",
            ),
            niri_leaf(9, "Move to Next Workspace", "move-window-to-workspace-down"),
            niri_leaf(10, "Move to Monitor Left", "move-window-to-monitor-left"),
            niri_leaf(11, "Move to Monitor Right", "move-window-to-monitor-right"),
        ],
    )
}

/// Pretty-print a Wayland app-id for the synthetic menu's top-level
/// label. Strips reverse-DNS prefix and capitalises:
/// - `com.mitchellh.ghostty` → `Ghostty`
/// - `org.kde.okular` → `Okular`
/// - `firefox-nightly` → `Firefox-nightly`
/// - empty → `App`
pub(crate) fn pretty_app_label(app_id: &str) -> String {
    let stripped = match app_id.rfind('.') {
        Some(idx) if app_id[..idx].contains('.') => &app_id[idx + 1..],
        _ => app_id,
    };
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return "App".to_string();
    }
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => "App".to_string(),
    }
}

/// Dispatch a synthetic menu click. `path` carries the action keyed
/// by dispatcher:
/// - `niri:<action>` runs `niri msg action <action>`.
/// - `niri:noop` is a no-op (used for parent submenu paths).
///
/// Future dispatchers (e.g. `kill:<sig>`, `xdg-open:<uri>`) plug in
/// the same way. The `wtype:<combo>` dispatcher (alpha.10's Edit
/// submenu) was removed in PR #44 — see `synthetic_menu` rationale.
async fn dispatch_synthetic(path: &str) -> Result<()> {
    let (dispatcher, action) = path
        .split_once(':')
        .with_context(|| format!("synthetic path missing dispatcher prefix: {path}"))?;
    match dispatcher {
        "niri" => dispatch_niri_action(action).await,
        // Spec 016 desktop-fallback: `xdg:<desktop-id>` launches the
        // resolved app's default action; `xdg-action:<desktop-id>:<action-id>`
        // launches a specific `[Desktop Action]`. Both re-resolve the
        // id against the trusted XDG application dirs at click time and
        // spawn the parsed Exec as argv (NO shell) — see `crate::desktop`.
        "xdg" => crate::desktop::launch_app(action).await,
        "xdg-action" => {
            let (id, action_id) = action.split_once(':').with_context(|| {
                format!("xdg-action path missing `<desktop-id>:<action-id>`: {path}")
            })?;
            crate::desktop::launch_action(id, action_id).await
        }
        other => anyhow::bail!("unknown synthetic dispatcher: {other}"),
    }
}

async fn dispatch_niri_action(action: &str) -> Result<()> {
    if action == "noop" {
        return Ok(());
    }
    // Run `niri msg action <action>` as a one-shot child. Ignore
    // stdout/stderr — niri's reply is irrelevant; we care only
    // about exit status.
    let status = tokio::process::Command::new("niri")
        .args(["msg", "action", action])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .with_context(|| format!("spawning niri msg action {action}"))?;
    if !status.success() {
        anyhow::bail!("niri msg action {action} exited {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(s: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(s).unwrap()
    }

    // ADR-0030 frame-selection policy.
    #[test]
    fn choose_frame_single_or_zero_window_uses_app_root() {
        // 0 or 1 top-level window: the app root is unambiguous, preserve
        // the proven single-window path regardless of title/active hits.
        assert_eq!(choose_frame(0, None, None), FrameChoice::AppRoot);
        assert_eq!(choose_frame(1, None, None), FrameChoice::AppRoot);
        assert_eq!(
            choose_frame(1, Some(op("/win/1")), Some(op("/win/2"))),
            FrameChoice::AppRoot
        );
    }

    #[test]
    fn choose_frame_title_hit_beats_active() {
        // Multi-window: deterministic title match wins over STATE_ACTIVE.
        assert_eq!(
            choose_frame(3, Some(op("/win/title")), Some(op("/win/active"))),
            FrameChoice::Frame(op("/win/title"))
        );
    }

    #[test]
    fn choose_frame_active_is_tiebreaker() {
        assert_eq!(
            choose_frame(2, None, Some(op("/win/active"))),
            FrameChoice::Frame(op("/win/active"))
        );
    }

    #[test]
    fn choose_frame_multiwindow_no_match_serves_placeholder() {
        // The bug case: several windows, none identifiable as focused.
        // Serving the app root would DFS an arbitrary window's menubar.
        assert_eq!(choose_frame(3, None, None), FrameChoice::NoMenu);
    }

    #[test]
    fn title_matches_is_exact_trimmed_only() {
        assert!(title_matches(
            "Mail — Firefox Nightly",
            "Mail — Firefox Nightly"
        ));
        assert!(title_matches("  Mail — Firefox  ", "Mail — Firefox"));
        // Distinct windows sharing a suffix must NOT match.
        assert!(!title_matches(
            "CapSolver — Firefox Nightly",
            "Mail — Firefox Nightly"
        ));
        // No substring/containment matching (would mispair same-app windows).
        assert!(!title_matches("Firefox Nightly", "Mail — Firefox Nightly"));
        assert!(!title_matches("", ""));
        assert!(!title_matches("", "Mail"));
    }

    #[test]
    fn menu_item_default_is_empty() {
        let m = MenuItem::default();
        assert_eq!(m.id, 0);
        assert!(m.children.is_empty());
        assert!(m.service.is_empty());
    }

    #[test]
    fn role_constants_match_atspi_wire_enum() {
        // at-spi2-core/atspi/atspi-constants.h `AtspiRole`. Verified
        // 2026-05-06 against okular 26.04 + Qt 6.11 menubar.
        assert_eq!(role::MENU, 33);
        assert_eq!(role::MENU_BAR, 34);
        assert_eq!(role::MENU_ITEM, 35);
        assert_eq!(role::SEPARATOR, 50);
    }

    #[test]
    fn menu_item_serializes_with_atspi_coords() {
        let m = MenuItem {
            id: 1,
            label: "File".to_string(),
            item_type: "submenu".to_string(),
            enabled: true,
            visible: true,
            service: ":1.42".to_string(),
            path: "/org/a11y/atspi/accessible/12".to_string(),
            ..Default::default()
        };
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains(r#""service":":1.42""#));
        assert!(s.contains(r#""path":"/org/a11y/atspi/accessible/12""#));
    }

    #[test]
    fn pretty_app_label_strips_reverse_dns_and_capitalises() {
        assert_eq!(pretty_app_label("com.mitchellh.ghostty"), "Ghostty");
        assert_eq!(pretty_app_label("org.kde.okular"), "Okular");
        assert_eq!(pretty_app_label("firefox-nightly"), "Firefox-nightly");
        assert_eq!(pretty_app_label(""), "App");
        assert_eq!(pretty_app_label("   "), "App");
    }

    #[test]
    fn synthetic_menu_honest_layout() {
        // PR #44: dropped Edit submenu. Layout is now Application +
        // Window only — both wired to real niri-IPC primitives.
        let m = synthetic_menu("com.mitchellh.ghostty");
        assert_eq!(m.label, "Ghostty");
        assert_eq!(m.item_type, "submenu");
        assert_eq!(m.service, SYNTHETIC_SERVICE);
        // Top-level: Application + Window. NO Edit.
        assert_eq!(m.children.len(), 2);
        assert_eq!(m.children[0].label, "Ghostty");
        assert_eq!(m.children[1].label, "Window");
        // Application submenu: Quit <Pretty>.
        assert_eq!(m.children[0].children.len(), 1);
        assert_eq!(m.children[0].children[0].label, "Quit Ghostty");
        assert_eq!(m.children[0].children[0].path, "niri:close-window");
        // Window: clickable leaves carry niri:<action> + the synthetic
        // service; separators are inert (empty service/path) groupers.
        for item in &m.children[1].children {
            if item.item_type == "separator" {
                assert!(item.service.is_empty() && item.path.is_empty());
                continue;
            }
            assert!(
                item.path.starts_with("niri:"),
                "Window leaf must use niri dispatcher; got {}",
                item.path
            );
            assert_eq!(item.service, SYNTHETIC_SERVICE);
        }
        // Wire-compat: same JSON shape as AT-SPI items.
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["service"], SYNTHETIC_SERVICE);
        assert_eq!(json["children"][1]["label"], "Window");
    }

    #[test]
    fn synthetic_menu_no_wtype_paths() {
        // Regression guard: PR #44 stripped wtype-driven items. If
        // someone reintroduces them they MUST update this test (and
        // think about whether the UX trap is back).
        for app_id in ["com.mitchellh.ghostty", "firefox-nightly", "org.kde.okular"] {
            let m = synthetic_menu(app_id);
            fn check_no_wtype(item: &MenuItem) {
                assert!(
                    !item.path.starts_with("wtype:"),
                    "synthetic menu must not contain wtype paths; got {} on {}",
                    item.path,
                    item.label
                );
                for c in &item.children {
                    check_no_wtype(c);
                }
            }
            check_no_wtype(&m);
        }
    }

    #[test]
    fn synthetic_menu_handles_empty_app_id() {
        let m = synthetic_menu("");
        assert_eq!(m.label, "App");
        assert_eq!(m.children.len(), 2);
    }

    #[test]
    fn niri_action_icon_maps_known_actions_only() {
        assert_eq!(niri_action_icon("close-window"), "window-close");
        assert_eq!(niri_action_icon("fullscreen-window"), "view-fullscreen");
        assert_eq!(niri_action_icon("move-window-to-workspace-down"), "go-down");
        assert_eq!(niri_action_icon("move-window-to-workspace-up"), "go-up");
        // Unmapped actions get NO icon — never a misleading one.
        assert_eq!(niri_action_icon("toggle-window-floating"), "");
        assert_eq!(niri_action_icon("noop"), "");
        assert_eq!(niri_action_icon("some-future-action"), "");
    }

    #[test]
    fn niri_leaf_carries_action_icon() {
        assert_eq!(
            niri_leaf(0, "Close", "close-window").icon_name,
            "window-close"
        );
        assert_eq!(
            niri_leaf(1, "Toggle Floating", "toggle-window-floating").icon_name,
            ""
        );
    }

    #[test]
    fn synthetic_window_submenu_layout_and_icons() {
        let w = synthetic_window_submenu();
        // (label, type, icon) for every row — locks the grouped layout.
        let rows: Vec<(&str, &str, &str)> = w
            .children
            .iter()
            .map(|c| (c.label.as_str(), c.item_type.as_str(), c.icon_name.as_str()))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("Close", "standard", "window-close"),
                ("Toggle Fullscreen", "standard", "view-fullscreen"),
                ("Toggle Floating", "standard", ""),
                ("", "separator", ""),
                ("Maximize Column", "standard", ""),
                ("Center Column", "standard", ""),
                ("Expand Column to Available Width", "standard", ""),
                ("", "separator", ""),
                ("Move to Previous Workspace", "standard", "go-up"),
                ("Move to Next Workspace", "standard", "go-down"),
                ("Move to Monitor Left", "standard", "go-previous"),
                ("Move to Monitor Right", "standard", "go-next"),
            ]
        );
        // child ids are sequential (the QML Repeater keys on them).
        for (i, c) in w.children.iter().enumerate() {
            assert_eq!(c.id, i as i32);
        }
    }

    #[test]
    fn niri_action_icon_maps_monitor_moves() {
        assert_eq!(
            niri_action_icon("move-window-to-monitor-left"),
            "go-previous"
        );
        assert_eq!(niri_action_icon("move-window-to-monitor-right"), "go-next");
        // Column ops have no clean standard icon.
        assert_eq!(niri_action_icon("maximize-column"), "");
        assert_eq!(niri_action_icon("center-column"), "");
    }

    #[test]
    fn menu_error_stale_round_trips_through_anyhow() {
        // FR-007 contract: `do_action` returns `Result<(), anyhow::Error>`,
        // but callers (the `atspi-click` CLI) need to discriminate
        // `MenuError::Stale` from generic failures so they can trigger
        // a snapshot refresh + exit 2. The contract is "embed via
        // `.into()`, recover via `downcast_ref::<MenuError>()`".
        let err: anyhow::Error = MenuError::Stale {
            service: ":1.42".to_string(),
            path: "/org/a11y/atspi/accessible/12".to_string(),
        }
        .into();
        let down = err
            .downcast_ref::<MenuError>()
            .expect("MenuError must downcast back out of anyhow::Error");
        assert_eq!(
            down,
            &MenuError::Stale {
                service: ":1.42".to_string(),
                path: "/org/a11y/atspi/accessible/12".to_string(),
            }
        );
        // Display includes both fields so log scrapers + the CLI's
        // stderr line stay greppable.
        let s = format!("{down}");
        assert!(s.contains(":1.42"), "Display omits service: {s}");
        assert!(
            s.contains("/org/a11y/atspi/accessible/12"),
            "Display omits path: {s}"
        );
        assert!(s.starts_with("MenuError::Stale"), "Display prefix: {s}");
    }

    #[test]
    fn normalize_app_id_strips_kde_double_prefix() {
        // FR-008: KDE apps publish `app_id = "org.kde.<app>"` on
        // Wayland but expose AT-SPI Name = "<app>" (e.g. "kate",
        // "Dolphin"). `normalize_app_id` must strip the reverse-DNS
        // prefix on both sides so the fuzzy match has matching
        // shapes to compare.
        assert_eq!(normalize_app_id("org.kde.kate"), "kate");
        assert_eq!(normalize_app_id("org.kde.dolphin"), "dolphin");
        assert_eq!(normalize_app_id("org.kde.okular"), "okular");
        // Case + whitespace fold to the same canonical form so the
        // accessible-Name capitalisation (`Dolphin`, `Kate`) and
        // the wayland-app_id casing (`org.kde.dolphin`) collapse.
        assert_eq!(normalize_app_id("Dolphin"), "dolphin");
        assert_eq!(normalize_app_id("  Kate  "), "kate");
    }

    #[test]
    fn normalize_app_id_preserves_single_dot_suffix() {
        // Anki ships as a Python wrapper that launches a Qt
        // subprocess called `anki.bin`. The bridge sees the
        // wrapper's PID via niri, but the AT-SPI accessible
        // registers under the subprocess. The fuzzy-match floor
        // is "3+ char short side, substring of long side", so we
        // need `normalize_app_id` to NOT strip `.bin` off as if it
        // were a reverse-DNS suffix (one dot, prefix has zero
        // dots → keep as-is).
        assert_eq!(normalize_app_id("anki.bin"), "anki.bin");
        assert_eq!(normalize_app_id("anki"), "anki");
        // Sanity: single-dot non-DNS identifiers stay whole. Two
        // dots → strip everything before the rightmost dot only
        // when the prefix itself contains a dot.
        assert_eq!(normalize_app_id("firefox-nightly"), "firefox-nightly");
        assert_eq!(normalize_app_id("foo.bar"), "foo.bar");
        assert_eq!(normalize_app_id("a.b.c"), "c");
    }

    #[test]
    fn fuzzy_app_match_recognises_kde_and_anki_pairings() {
        // FR-008 happy paths against the three v1 reference apps:

        // kate: niri-reported `org.kde.kate` → normalize → "kate".
        // AT-SPI Name `"kate"` → normalize → "kate". Equal strings
        // satisfy the substring predicate trivially.
        assert!(fuzzy_app_match(
            &normalize_app_id("org.kde.kate"),
            &normalize_app_id("kate")
        ));

        // dolphin: same shape, capitalised AT-SPI Name.
        assert!(fuzzy_app_match(
            &normalize_app_id("org.kde.dolphin"),
            &normalize_app_id("Dolphin")
        ));

        // Anki subprocess: niri reports `anki` (wrapper script's
        // own app_id), AT-SPI registers `anki.bin` (the Qt
        // subprocess). Short = "anki" (4 chars, ≥ 3), long =
        // "anki.bin" (contains "anki") → match.
        assert!(fuzzy_app_match(
            &normalize_app_id("anki"),
            &normalize_app_id("anki.bin")
        ));
        // Argument order must not matter.
        assert!(fuzzy_app_match(
            &normalize_app_id("anki.bin"),
            &normalize_app_id("anki")
        ));
    }

    #[test]
    fn fuzzy_app_match_rejects_short_and_empty() {
        // 2-char fragments are too generic. `qt` would otherwise
        // collide with anything Qt-named (qBittorrent, KAlgebra,
        // etc.) and misroute the menubar.
        assert!(!fuzzy_app_match("qt", "qtcreator"));
        assert!(!fuzzy_app_match("ok", "okular"));

        // Empty sides never match — `normalize_app_id` can return
        // empty on whitespace-only AT-SPI Names, and the caller
        // depends on the helper saying "no" rather than panicking.
        assert!(!fuzzy_app_match("", "okular"));
        assert!(!fuzzy_app_match("kate", ""));
        assert!(!fuzzy_app_match("", ""));

        // Non-substring disjoint pairs.
        assert!(!fuzzy_app_match("firefox", "chromium"));
    }

    #[test]
    fn empty_menubar_triggers_synthetic_fallback() {
        // FR-004: a MENU_BAR walked from a GTK4 `GtkPopoverMenuBar`
        // (Nautilus 45+, etc.) surfaces with `children: []` because
        // GTK4 defers child realisation until popup. The fallback
        // predicate must say "yes, swap in synthetic" for that shape.
        let empty_menubar = MenuItem {
            id: 0,
            label: "menubar".to_string(),
            item_type: "submenu".to_string(),
            enabled: true,
            visible: true,
            service: ":1.42".to_string(),
            path: "/org/a11y/atspi/accessible/menubar".to_string(),
            ..Default::default()
        };
        assert!(empty_menubar.children.is_empty());
        assert!(
            menubar_is_empty(&empty_menubar),
            "empty-children menubar must route into synthetic fallback"
        );

        // Conversely, a walked menubar with at least one top-level
        // entry stays on the AT-SPI path — no fallback.
        let populated = MenuItem {
            children: vec![MenuItem {
                id: 0,
                label: "File".to_string(),
                item_type: "submenu".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(!menubar_is_empty(&populated));
    }

    #[tokio::test]
    async fn atspi_client_invalidate_is_idempotent() {
        // FR-006: AtspiClient::invalidate must be safe to call on an
        // empty cache (e.g. before the first connection() succeeded)
        // — the watcher hits this path on every a11y bus restart.
        let c = AtspiClient::new();
        c.invalidate().await;
        c.invalidate().await;
    }

    #[test]
    fn atspi_client_clone_is_cheap() {
        // The watcher + active loop share one AtspiClient — clone is
        // just an Arc bump. Compile-time check; if Clone is removed
        // from AtspiClient this test fails to build.
        fn _assert_clone<T: Clone + Send + Sync + 'static>() {}
        _assert_clone::<AtspiClient>();
    }

    // ── Spec 009 FR-001 — recursive Qt wrapper-flatten tests ─────────

    fn leaf(label: &str) -> MenuItem {
        MenuItem {
            label: label.to_string(),
            item_type: "standard".to_string(),
            enabled: true,
            visible: true,
            ..Default::default()
        }
    }

    fn submenu(label: &str, children: Vec<MenuItem>) -> MenuItem {
        MenuItem {
            label: label.to_string(),
            item_type: "submenu".to_string(),
            enabled: true,
            visible: true,
            children,
            ..Default::default()
        }
    }

    /// Wrapper = empty-label submenu wrapping real children. The
    /// shape Qt6 emits at every level of its accessibility tree.
    fn qt_wrap(real_children: Vec<MenuItem>) -> MenuItem {
        submenu("", real_children)
    }

    #[test]
    fn flatten_strips_single_empty_wrapper() {
        // File (MENU_ITEM, 1 child)
        //   └── "" (MENU, [Open, Save])
        let mut item = submenu("File", vec![qt_wrap(vec![leaf("Open"), leaf("Save")])]);
        flatten_qt_wrapper(&mut item);
        assert_eq!(item.children.len(), 2);
        assert_eq!(item.children[0].label, "Open");
        assert_eq!(item.children[1].label, "Save");
        // Re-id contract: post-flatten ids are sequential.
        assert_eq!(item.children[0].id, 0);
        assert_eq!(item.children[1].id, 1);
    }

    #[test]
    fn flatten_is_idempotent_on_already_flat_tree() {
        let mut item = submenu("File", vec![leaf("Open"), leaf("Save")]);
        let before = item.clone();
        flatten_qt_wrapper(&mut item);
        // No structural change.
        assert_eq!(item.children.len(), before.children.len());
        assert_eq!(item.children[0].label, "Open");
        assert_eq!(item.children[1].label, "Save");
    }

    #[test]
    fn flatten_rejects_multi_child_root() {
        // {label: "X", children: [{empty MENU with kids}, {real "Other"}]}
        // is NOT a wrapper case — the real "Other" sibling means the
        // empty MENU is intentional. MUST NOT flatten.
        let mut item = submenu("X", vec![qt_wrap(vec![leaf("Hidden")]), leaf("Other")]);
        flatten_qt_wrapper(&mut item);
        assert_eq!(item.children.len(), 2);
        assert_eq!(item.children[0].label, ""); // wrapper preserved
        assert_eq!(item.children[1].label, "Other");
    }

    #[test]
    fn flatten_skips_empty_leaf() {
        // {label: "", children: []} is a leaf separator-like item;
        // there's nothing to flatten away.
        let mut item = leaf("");
        flatten_qt_wrapper(&mut item);
        assert!(item.children.is_empty());
    }

    #[test]
    fn flatten_skips_named_wrapper() {
        // The flatten predicate requires children[0].label.is_empty().
        // A label-bearing child is a real submenu, never a wrapper.
        let mut item = submenu("File", vec![submenu("Submenu", vec![leaf("Item")])]);
        flatten_qt_wrapper(&mut item);
        assert_eq!(item.children.len(), 1);
        assert_eq!(item.children[0].label, "Submenu"); // preserved
    }

    #[test]
    fn flatten_preserves_toggle_state() {
        // Toggle/radio leaves carry toggle_type + toggle_state. The
        // flatten ONLY moves children up; it does NOT mutate leaves.
        let mut toggle = leaf("Bold");
        toggle.toggle_type = "checkmark".to_string();
        toggle.toggle_state = 1;
        let mut item = submenu("Format", vec![qt_wrap(vec![toggle.clone()])]);
        flatten_qt_wrapper(&mut item);
        assert_eq!(item.children.len(), 1);
        assert_eq!(item.children[0].label, "Bold");
        assert_eq!(item.children[0].toggle_type, "checkmark");
        assert_eq!(item.children[0].toggle_state, 1);
    }

    #[test]
    fn fetch_menu_tree_recursion_invariant_three_levels() {
        // Recursive composition of flatten matches the contract in
        // contracts/recursive-flatten.md: every level is wrapper-
        // stripped by the time it is returned upwards.
        //
        // Synthetic shape mirrors shadPS4QtLauncher's
        //   View > Game List Mode > [List, Grid, Flat]
        // before flatten:
        //   Game List Mode
        //     └── ""  (wrapper)
        //           ├── List
        //           ├── Grid
        //           └── Flat
        // After: Game List Mode → [List, Grid, Flat] (wrapper gone).
        let mut game_list_mode = submenu(
            "Game List Mode",
            vec![qt_wrap(vec![leaf("List"), leaf("Grid"), leaf("Flat")])],
        );
        flatten_qt_wrapper(&mut game_list_mode);
        assert_eq!(game_list_mode.children.len(), 3);
        assert_eq!(game_list_mode.children[0].label, "List");
        assert_eq!(game_list_mode.children[1].label, "Grid");
        assert_eq!(game_list_mode.children[2].label, "Flat");

        // Compose into View; pre-flatten View has one wrapper child
        // whose grandchildren are three sibling submenus, one of
        // which (Game List Mode, above) is already flat.
        let mut view = submenu(
            "View",
            vec![qt_wrap(vec![
                leaf("Show Game List"),
                game_list_mode.clone(),
                leaf("Themes"),
            ])],
        );
        flatten_qt_wrapper(&mut view);
        assert_eq!(view.children.len(), 3);
        assert_eq!(view.children[0].label, "Show Game List");
        assert_eq!(view.children[1].label, "Game List Mode");
        // Grandchild access — Game List Mode keeps its already-flat
        // children after the parent's flatten lifts the wrapper.
        assert_eq!(view.children[1].children.len(), 3);
        assert_eq!(view.children[1].children[0].label, "List");
    }
}
