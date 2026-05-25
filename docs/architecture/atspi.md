# AT-SPI pipeline

The full life of a menu item from app to bar, as actually shipped
since v0.3 / [ADR-0024](../adr/ADR-0024-atspi-substrate.md).

For the historical DBusMenu/Registrar pipeline (the design v0.1..v0.2
attempted but Qt6 on niri never registered against), see
[`dbusmenu.md`](./dbusmenu.md).

## Why this substrate

DBusMenu requires apps to call `RegisterWindow` against a registrar
service. Qt6's auto-registration only fires on compositors implementing
`org_kde_kwin_appmenu_manager` (KWin only). niri, Hyprland, Sway,
COSMIC: none implement it. Result: no Qt app on niri ever registered
against the v0.2 bridge, regardless of correctness.

AT-SPI is the cross-toolkit substrate that already works:

- Qt apps load `qtatspi` at `QApplication` startup when
  `QT_ACCESSIBILITY=1` is set (the NixOS module ships this).
- Qt's `QMenuBar` is exposed under `Role::MenuBar` automatically.
- GTK apps expose menus via ATK → AT-SPI without extra config.
- Anki, Okular, Firefox, GIMP, Files all surface menubars identically.
- No protocol cooperation required from the compositor.

## Connection topology

AT-SPI lives on its own dedicated D-Bus bus (NOT the session bus) to
keep accessibility traffic isolated.

1. Query `org.a11y.Bus` on the SESSION bus for the a11y bus address
   via `GetAddress()`.
2. Connect to that address (typically a UNIX socket like
   `unix:abstract=/tmp/at-spi2-bus-XXXXX/socket`).
3. Use the registry root at well-known service
   `org.a11y.atspi.Registry`, path
   `/org/a11y/atspi/accessible/root`. Its children are the registered
   Application objects, one per a11y-aware app.

## Walking the tree

`org.a11y.atspi.Accessible` interface methods the bridge uses:

| Method | Purpose |
|---|---|
| `GetChildAtIndex(i: i32) → (s, o)` | Returns `(busName, path)` of the i-th child — every accessible object lives on its own object path within a single application's bus connection. |
| `Property: ChildCount (i)` | int32 count of children. |
| `Property: Name (s)` | Display name (e.g. "File", "Edit"). |
| `GetRole() → u` | Wire-level role enum from at-spi2-core's `atspi-constants.h`. Stable across releases. |
| `GetState() → au` | State ints; the bridge uses `ENABLED=20`, `VISIBLE=37`, `FOCUSABLE=10`. |
| `GetApplication() → (s, o)` | Owning app's accessible (root of per-app subtree). |

Click forwarding uses the `org.a11y.atspi.Action` interface:

| Method | Purpose |
|---|---|
| `DoAction(i: i32) → b` | Invoke the i-th action. Index 0 is "click" by convention (verified against Qt's `qtatspi`). |
| `Property: NActions (i)` | Action count. |

Role IDs the bridge dispatches on (verified live 2026-05-06 against
okular 26.04 + Qt 6.11):

```
CHECK_MENU_ITEM    8
MENU              33
MENU_BAR          34
MENU_ITEM         35
RADIO_MENU_ITEM   45
SEPARATOR         50
TEAR_OFF_MENU_ITEM 60
```

Maximum tree-walk depth is capped at `MAX_FIND_DEPTH = 8` to prevent
runaway walks on malformed trees.

## PID matching

niri's `WindowFocusChanged` event gives the bridge a PID. AT-SPI
doesn't key on PID directly, so the bridge:

1. Walks Registry root's children (each is an Application).
2. For each, resolves its bus name to a PID via the a11y bus's
   `org.freedesktop.DBus.GetConnectionUnixProcessID(name)`.
3. Matches against niri's focused PID. First hit wins.

## Frame-scoped resolution (spec 016 / ADR-0030)

When multiple top-level windows live behind the same PID (Anki's
profile picker vs. main window, Firefox's multi-window, kate's
"open new window"), PID alone is ambiguous. Spec 016 added a
focused-window-title filter:

1. The niri event also carries the focused window's `title`.
2. After PID-matching the AT-SPI Application, the bridge enumerates
   the app's accessible frame children and prefers the frame whose
   `Name` matches the niri-reported title.
3. The menubar walk starts under that frame, not the app root.

This is what fixes the v1.0.20..v1.0.24 "wrong menu shown for the
focused window" loop. See [`ADR-0030`](../adr/ADR-0030-frame-scoped-menu-resolution.md).

## Learned no-menubar skip (v1.0.9 / ADR-0029)

Terminals, X11-under-xwayland-satellite apps, and Chrome don't expose
a menubar via AT-SPI. Walking them drains the `FETCH_BUDGET` on every
focus event and freezes the bar.

The bridge tracks two outcomes per `app_id`:

- **Expensive no-menubar walk** (≥ `EXPENSIVE_WALK`) — verdict is
  permanent for the process lifetime. Reproduces the v1.0.6..v1.0.8
  hardcoded skip list from the *outcome*, not a name.
- **Cheap no-menubar walk** — verdict expires after `RECHECK_TTL`, so
  apps that lazy-build their menubar self-heal on re-walk.

A walk that finds a real menubar (`forget`) drops any verdict.

See [`ADR-0029`](../adr/ADR-0029-learned-no-menubar-skip.md).

## In the proxy

The bridge serialises the walked menu tree into a fixed proxy at
`org.noctalia.AppMenu /org/noctalia/AppMenu/Active`. The Quickshell
plugin subscribes to that path — no per-app Registrar lookup, no
windowId resolution, no protocol negotiation in QML.
