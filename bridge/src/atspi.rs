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

/// Maximum recursion depth for fetching menu items once we've
/// found a `MenuBar`. Real menubars rarely nest more than 3-4 levels;
/// 6 gives slack for pathological apps without runaway cost.
const MAX_FETCH_DEPTH: u32 = 6;

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
            return Ok(Some((service, path)));
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
                if normalized_name.is_empty() {
                    continue;
                }
                let (short, long) = if normalized_name.len() < normalized_hint.len() {
                    (&normalized_name, &normalized_hint)
                } else {
                    (&normalized_hint, &normalized_name)
                };
                if short.len() >= 3 && long.contains(short.as_str()) {
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
        if n.is_empty() {
            continue;
        }
        let (short, long) = if n.len() < normalized_hint.len() {
            (&n, &normalized_hint)
        } else {
            (&normalized_hint, &n)
        };
        if short.len() >= 3 && long.contains(short.as_str()) {
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

    for i in 0..count {
        let (child_service, child_path) = match proxy.get_child_at_index(i).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        match Box::pin(fetch_menu_tree(
            a11y,
            &child_service,
            &child_path.as_ref(),
            cur_depth + 1,
        ))
        .await
        {
            Ok(mut child) => {
                child.id = i;
                item.children.push(child);
            }
            Err(_) => continue,
        }
    }

    // Qt wraps every MENU_ITEM's popup in an unnamed MENU child:
    //   File (MENU_ITEM, label "File", 1 child)
    //     └── "" (MENU, label "", N children) ← popup wrapper
    //           ├── Open (MENU_ITEM)
    //           └── Save (MENU_ITEM)
    //
    // The QML widget renders `item.children` directly under each
    // top-level button — without flattening, the popup would show a
    // single empty-label entry with the real items hidden one level
    // deeper. Detect the wrapper shape and pull the grandchildren up.
    if item.children.len() == 1
        && item.children[0].label.is_empty()
        && !item.children[0].children.is_empty()
    {
        let grandchildren = std::mem::take(&mut item.children[0].children);
        item.children = grandchildren;
        // Re-id the new top-level so QML's stable-id logic still works.
        for (i, c) in item.children.iter_mut().enumerate() {
            c.id = i as i32;
        }
    }

    Ok(item)
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
) -> Result<Option<MenuItem>> {
    const FETCH_BUDGET: std::time::Duration = std::time::Duration::from_millis(3000);
    match tokio::time::timeout(
        FETCH_BUDGET,
        fetch_menubar_for_pid_inner(client, pid, app_id_hint),
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
) -> Result<Option<MenuItem>> {
    let a11y = client.connection().await?;
    let app = match find_app_for_pid(&a11y, pid, app_id_hint).await? {
        Some(a) => a,
        None => return Ok(None),
    };
    let menubar = match find_menubar(&a11y, &app.0, &app.1.as_ref(), 0).await? {
        Some(mb) => mb,
        None => return Ok(None),
    };
    let tree = fetch_menu_tree(&a11y, &menubar.0, &menubar.1.as_ref(), 0).await?;
    // FR-004: GTK4 `GtkPopoverMenuBar` exposes MENU_BAR with zero
    // children pre-popup. Returning the empty tree would render an
    // empty bar; surfacing None lets active.rs substitute the
    // synthetic pseudo-menu.
    if tree.children.is_empty() {
        tracing::debug!(
            pid,
            "menubar walk returned zero children — caller should fall back to synthetic menu"
        );
        return Ok(None);
    }
    Ok(Some(tree))
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
                    "a11y IsEnabled observed false; re-enabling + invalidating AT-SPI cache"
                );
                client.invalidate().await;
                if let Err(e) = enable_a11y().await {
                    tracing::warn!(error = ?e, "re-enable a11y after IsEnabled flip failed");
                }
            }
            Err(e) => {
                tracing::debug!(
                    error = ?e,
                    "a11y IsEnabled probe failed (bus restart in flight?); invalidating cache"
                );
                client.invalidate().await;
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
pub async fn do_action(service: &str, path: &str) -> Result<()> {
    if service == SYNTHETIC_SERVICE {
        return dispatch_synthetic(path).await;
    }
    let a11y = connect_a11y().await?;
    let proxy_path: ObjectPath<'_> = path
        .try_into()
        .with_context(|| format!("parsing AT-SPI path {path}"))?;
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

/// `niri:<action>` leaf factory. ID is the parent's child index.
fn niri_leaf(id: i32, label: &str, action: &str) -> MenuItem {
    MenuItem {
        id,
        label: label.to_string(),
        item_type: "standard".to_string(),
        enabled: true,
        visible: true,
        icon_name: String::new(),
        toggle_type: String::new(),
        toggle_state: 0,
        service: SYNTHETIC_SERVICE.to_string(),
        path: format!("niri:{action}"),
        children: Vec::new(),
    }
}

/// Submenu wrapping `children` under `label`. Submenu's own click
/// path is the documented `niri:noop` so accidental leaf-style click
/// (which the QML widget guards against anyway) is a no-op.
fn synthetic_submenu(id: i32, label: &str, children: Vec<MenuItem>) -> MenuItem {
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

/// Window submenu — universal niri-IPC actions.
fn synthetic_window_submenu() -> MenuItem {
    synthetic_submenu(
        1,
        "Window",
        vec![
            niri_leaf(0, "Close", "close-window"),
            niri_leaf(1, "Toggle Fullscreen", "fullscreen-window"),
            niri_leaf(2, "Toggle Floating", "toggle-window-floating"),
            niri_leaf(3, "Move to Next Workspace", "move-window-to-workspace-down"),
            niri_leaf(
                4,
                "Move to Previous Workspace",
                "move-window-to-workspace-up",
            ),
        ],
    )
}

/// Pretty-print a Wayland app-id for the synthetic menu's top-level
/// label. Strips reverse-DNS prefix and capitalises:
/// - `com.mitchellh.ghostty` → `Ghostty`
/// - `org.kde.okular` → `Okular`
/// - `firefox-nightly` → `Firefox-nightly`
/// - empty → `App`
fn pretty_app_label(app_id: &str) -> String {
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
        // Window leaves all carry niri:<action> paths — never wtype.
        for leaf in &m.children[1].children {
            assert!(
                leaf.path.starts_with("niri:"),
                "Window leaf must use niri dispatcher; got {}",
                leaf.path
            );
            assert_eq!(leaf.service, SYNTHETIC_SERVICE);
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
}
