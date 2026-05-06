# ADR-0024 — Replace DBusMenu/Registrar with AT-SPI menubar walker

- **Status:** accepted
- **Date:** 2026-05-06
- **Deciders:** Pedro H S Balbino
- **Supersedes:** ADR-0022 (bridge-owns-registrar), ADR-0023 (dbusmenu-fetch-on-focus)
- **Tracking PR / branch:** `32-atspi-walker`, release tag `v0.3.0-alpha.1`

## Context

v0.2.0-alpha.1 shipped a complete DBusMenu mirror: the bridge owned
`com.canonical.AppMenu.Registrar`, fetched `GetLayout(0, -1, [])` from
each registered Qt/GTK app on focus change, and re-published the parsed
tree under `active.json`. The architecture was correct, the
implementation passed all unit tests, and the QML widget rendered
correctly when fed a populated tree.

It did not work for any of Pedro's real applications.

The blocker turned out to be ecosystem-level, not a bug in our code.
Qt6's automatic DBusMenu registration is gated on the
`org_kde_kwin_appmenu_manager` Wayland protocol, which only KWin
implements. niri (Pedro's compositor), Hyprland, Sway, and COSMIC do
not — and there is no upstream movement to add it. Pedro's Anki,
Okular, Krita, and dolphin all run with no menubar export, regardless
of how correct the bridge's registrar implementation is.

We considered three paths forward:

1. **Wait for cross-compositor `org_kde_kwin_appmenu_manager` adoption.**
   Open-ended, no upstream timeline. niri's maintainer has explicitly
   declined adopting KWin-specific Wayland protocols.
2. **Fork Qt** to remove the gate. Untenable maintenance burden — Qt
   binary updates would require rebuilding the patched fork.
3. **Switch substrate to AT-SPI.** AT-SPI is the cross-toolkit
   accessibility bus that Qt and GTK already implement for screen
   readers (Orca, NVDA on Linux, etc.). Apps expose their full widget
   tree — including `QMenuBar` / `GtkMenuBar` — for free, on every
   compositor, with zero protocol-cooperation requirement.

## Decision

Replace the v0.2 DBusMenu/Registrar substrate with an AT-SPI walker.

The bridge:

1. Calls `org.a11y.Bus.GetAddress` on the session bus to discover the
   a11y bus address.
2. Connects to the a11y bus, sets `org.a11y.Status.IsEnabled = true`
   so Qt's accessibility bridge actually registers (Orca normally
   flips this on stock GNOME; niri ships no AT, so the bridge owns
   it).
3. On every niri focus change: walks
   `org.a11y.atspi.Registry`'s children, matches the focused PID via
   the a11y bus's `GetConnectionUnixProcessID` (different name → PID
   mapping than the session bus), DFS-searches for an accessible
   with role `MENU_BAR = 34`, and walks its subtree into the same
   JSON shape v0.2 produced.
4. Click forwarding: a separate `atspi-click <service> <path>`
   subcommand spawns a one-shot child process that calls
   `org.a11y.atspi.Action.DoAction(0)` on the addressed accessible.
   qtatspi convention: action index 0 is "click".

The QML widget needs zero structural changes — `active.json`'s `menu`
field keeps the same shape (`{ id, label, type, enabled, visible,
icon_name, toggle_type, toggle_state, service, path, children }`).
Only the per-item `service` and `path` semantics changed: they now
address AT-SPI accessibles, not DBusMenu items.

## Consequences

### Positive

- **Compositor-agnostic.** Works on niri, Hyprland, Sway, COSMIC,
  KDE, GNOME, Cosmic — the protocol stack is shared by every Linux
  desktop with screen-reader support, which is all of them.
- **Toolkit-agnostic.** Qt6, Qt5, GTK3, GTK4, GTK2 all expose
  `MENU_BAR` accessibles. Electron (`--force-accessibility`),
  Firefox (a11y enabled by default), and even Tk surface menus.
- **Zero registrar cooperation.** Apps don't have to "register" with
  us — they're already on the a11y bus by virtue of being a11y-aware.
- **Existing apps Just Work** on a NixOS rebuild with
  `services.gnome.at-spi2-core.enable = true` and
  `QT_ACCESSIBILITY=1` set system-wide.

### Negative

- **Tied to AT-SPI's lifecycle.** If the a11y bus dies (rare, but
  possible), the bridge stops mirroring until the bus restarts. v0.2
  had the same exposure on the registrar.
- **Item identifiers are AT-SPI accessible paths**, which can be
  recycled when an app rebuilds its widget tree. Click forwarding
  needs to fetch the tree and click in one step (no caching across
  focus changes); v0.2's DBusMenu integer ids had the same property
  in practice.
- **No `LayoutUpdated` signal.** AT-SPI exposes `children-changed`
  on individual accessibles; we'd subscribe per-app to detect menus
  that mutate while focused. v0.3.0-alpha.1 fetches eagerly per
  focus change — same as v0.2's `LayoutUpdated`-deferred posture.
- **DBusMenu code path retired but not deleted yet.** v0.3.x will
  prune `bridge/src/dbusmenu.rs` and `bridge/src/registrar.rs` once
  the AT-SPI path has soaked under Pedro's daily use.

### Wire-level details verified live (2026-05-06)

- AT-SPI role enum (`AtspiRole` from `atspi-constants.h`) — wire
  values, NOT pyatspi's older numeric mapping:
  - `MENU = 33`, `MENU_BAR = 34`, `MENU_ITEM = 35`,
    `SEPARATOR = 50`, `CHECK_MENU_ITEM = 8`, `RADIO_MENU_ITEM = 45`.
- AT-SPI state enum (`AtspiStateType`):
  - `ENABLED = 8`, `SENSITIVE = 24`, `VISIBLE = 31`, `CHECKED = 4`,
    `SHOWING = 25`. Bitmask spans two `u32` words.
- Qt wraps every `MENU_ITEM`'s popup in an unnamed `MENU` child.
  Walker flattens the wrapper so the QML widget renders the actual
  items, not an empty placeholder.

### Failure modes (catalogued for future debugging)

1. **`org.a11y.Status.IsEnabled = false`.** Bridge flips it to true
   at startup; if `enable_a11y()` errors out, log a warning and
   proceed (Qt apps will silently not register).
2. **App doesn't have AT-SPI integration loaded.** Common for
   Electron without `--force-accessibility`, niche toolkits.
   `find_app_for_pid` returns `None`; the QML widget renders the
   v0.1 placeholder.
3. **App has a `MENU_BAR` but the search ran out of depth.**
   `MAX_FIND_DEPTH = 8` levels — should cover every practical UI.
   Pathological cases return `None`.

## References

- [AT-SPI2 specification](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/)
- [`atspi-constants.h`](https://gitlab.gnome.org/GNOME/at-spi2-core/-/blob/master/atspi/atspi-constants.h)
  — authoritative role + state enum
- [Qt accessibility documentation](https://doc.qt.io/qt-6/accessible.html)
- ADR-0022 — bridge-owns-registrar (superseded)
- ADR-0023 — dbusmenu-fetch-on-focus (superseded)
- Spec 003 — `bridge-atspi-substrate` (this work's design doc)
