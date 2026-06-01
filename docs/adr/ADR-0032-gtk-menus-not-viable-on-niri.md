# ADR-0032 — `org.gtk.Menus` is not a viable menu substrate on niri (measured)

- **Status:** accepted
- **Date:** 2026-05-29
- **Deciders:** Pedro H S Balbino
- **Related:** ADR-0024 (AT-SPI substrate), ADR-0031 (desktop fallback)
- **Closes:** the "`org.gtk.Menus` / `GMenuModel` substrate" ladder item
  (the substrate slotted "above the desktop fallback, below AT-SPI")

## Context

The forward-work ladder listed an `org.gtk.Menus` / `GMenuModel` D-Bus
substrate as a candidate for getting *real* menus from GTK apps that
export a menubar model over D-Bus, rather than falling back to the
identity-derived desktop fallback (ADR-0031). The theory: GTK's
`GtkApplication` can export a menubar `GMenuModel` over the `org.gtk.Menus`
interface (with actions over `org.gtk.Actions`), keyed by a bus name +
object path derived from the app id — so a bridge could subscribe to that
model and re-publish it, scoped above the fallback.

Before designing the substrate, the assumption was measured live rather
than assumed (the project's standing discipline — see ADR-0024's pivot,
which was itself driven by a measurement that Qt6 never registers DBusMenu
on niri).

## Measurement (live, 2026-05-29, niri 26.04, session bus)

Deep object-tree introspection (`busctl --user tree <svc>` → introspect
every path) of every GTK app present on the session bus:

| App | `org.gtk.Application` | `org.gtk.Actions` | `org.gtk.Menus` |
|---|---|---|---|
| `org.gnome.baobab` | ✓ | ✓ | **✗** |
| `org.gnome.seahorse.Application` | ✓ | ✓ | **✗** |
| `org.gnome.Connections` | ✓ | ✓ | **✗** |
| `com.github.FontManager.FontManager` | ✓ | ✓ | **✗** |
| `org.gnome.Polari` | ✓ | ✓ | **✗** |

**0 of 5 GTK apps export `org.gtk.Menus` anywhere in their object tree.**
Every one exports `org.gtk.Actions` (an action group) at its app-id path,
but none exports a menubar `GMenuModel`. Electron (Obsidian, Feishin),
Chromium, Firefox, libcosmic and terminals are not GTK-menu-model apps at
all, so they are out of scope by construction.

## Why

1. **GTK4 has no app menubar.** Modern GTK apps render their menus in-window
   (header-bar / hamburger `GtkPopoverMenu`), built from a `GMenuModel`
   that is **consumed locally and never exported** as a menubar. There is
   nothing on `org.gtk.Menus` to subscribe to — the same "the app does not
   expose a machine-readable menubar" wall that AT-SPI hits (ADR-0024),
   reached from the other side.
2. **No Wayland path-discovery channel even if a model existed.** GTK
   advertises an exported menubar's object path to the *shell* via X11
   window properties (`_GTK_MENUBAR_OBJECT_PATH`, `_GTK_APPLICATION_…`).
   On native Wayland there is no equivalent property an external bridge can
   read per-focused-window without compositor cooperation — the identical
   compositor-protocol gap that retired the DBusMenu/Registrar path
   (ADR-0024). So the discovery half is also a dead end for native-Wayland
   clients, independent of (1).

## Decision

**Do not implement an `org.gtk.Menus` substrate.** It would add a
subscription-model D-Bus reader (the `org.gtk.Menus` protocol is layered
and stateful — `Start([groups]) → items`, group/subscription bookkeeping,
`org.gtk.Actions.DescribeAll` + `Activate` for clicks) for **zero measured
coverage** on a real niri desktop. The desktop fallback (ADR-0031) already
gives these apps an honest, useful menu (name + `.desktop` actions + niri
window controls), and AT-SPI (ADR-0024) already captures the apps that
*do* expose a real menubar (Qt6/GTK with the a11y bridge — Anki, Okular,
Kate, …).

The menu-source ladder is unchanged:

```
AT-SPI menubar  →  desktop fallback  →  empty
   (ADR-0024)        (ADR-0031)
```

## Revisit conditions

Reopen only if **both** hold:

1. A class of apps Pedro actually runs begins exporting a menubar
   `GMenuModel` over `org.gtk.Menus` (re-run the `busctl` tree scan; the
   table above is the baseline), **and**
2. their object path is discoverable per focused window — either because
   they are **XWayland** (then `xprop _GTK_MENUBAR_OBJECT_PATH` on the
   focused X11 window is readable, a narrow micro-enhancement), or niri
   grows a menu-advertisement protocol (it has declined KWin-specific
   appmenu protocols before — unlikely).

A narrow XWayland-only reader is the only fragment with any future ROI,
and even that is marginal — XWayland GTK menubar apps are rare. It is
explicitly **not** scheduled.

## Consequences

- **Positive:** a major ladder item is closed with evidence instead of
  left as an open "big substrate TODO" that would cost a multi-hundred-LoC
  stateful D-Bus reader for no coverage. Future agents read this instead of
  rediscovering it.
- **Positive:** reinforces the project's measure-first rule — the decision
  is backed by a reproducible `busctl` scan, not a guess.
- **Negative / cost:** GTK apps that hypothetically *could* gain an
  exported menubar later stay on the desktop fallback until the revisit
  conditions are met. Acceptable — the fallback is honest and useful.

## Update (2026-06-01) — Firefox ≥ 138 native menu does NOT reopen this

The original rationale (§Why, §Measurement) said Firefox "is not a
GTK-menu-model app". That is now stale and is corrected here, **without
changing the decision**. Verified against the Firefox source tree
(`mozilla-firefox/cedar` `widget/gtk/NativeMenuGtk.cpp` + `DBusMenu.cpp`,
Bugzilla 1883184 → `125 Branch`, 1956707 → `138 Branch`):

- Firefox ≥ 138 ships a *native* global menu, but it exports
  **`com.canonical.dbusmenu`** directly (built via `libdbusmenu-glib`) —
  **not** `org.gtk.Menus` / `GMenuModel`. So it never touches the
  `org.gtk.Menus` substrate this ADR rejected; the rejection stands.
- It is **opt-in** — `widget.gtk.global-menu.enabled`,
  `widget.gtk.global-menu.wayland.enabled`,
  `widget.gtk.native-context-menus` all default **`false`** in every
  version (138 only made the Wayland *path* functional, not default-on).
- On Wayland it activates **only** with a compositor binding
  `org_kde_kwin_appmenu_manager` **and** a `com.canonical.AppMenu.Registrar`
  owner present. **niri provides neither** → the native path is a no-op;
  Firefox advertises nothing on the session bus (confirmed live, FF 153).
- helloSystem's `gmenudbusmenuproxy` translates the GTK-*module*
  `org.gtk.Menus` export (generic GTK apps / LibreOffice) → dbusmenu;
  Firefox's *own* native path bypasses it and speaks dbusmenu directly.

**Net:** on niri, Firefox's menu substrate remains **AT-SPI**
(`accessibility.force_disabled = 0`, ADR-0024) at every version. The
revisit conditions (§Revisit) are unchanged — and would point at a future
**dbusmenu/registrar** reader (which ADR-0024 retired), not an
`org.gtk.Menus` reader. See the verified per-toolkit matrix in `CLAUDE.md`.

## References

- ADR-0024 — AT-SPI substrate (the original measure-first pivot)
- ADR-0031 — desktop fallback (covers these apps today)
- [GMenuModel exporter](https://docs.gtk.org/gio/class.DBusConnection.html#exporting-a-menu-model)
  / `org.gtk.Menus` + `org.gtk.Actions` D-Bus interfaces
- Reproduce: `busctl --user tree <app-bus-name>` then
  `busctl --user introspect <name> <path> | grep org.gtk.Menus`
