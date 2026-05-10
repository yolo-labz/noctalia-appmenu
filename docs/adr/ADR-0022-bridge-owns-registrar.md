# ADR-0022 — bridge owns the AppMenu Registrar bus name

- **Status:** Accepted (2026-05-05)
- **PR:** #29
- **Released in:** v0.2.0-alpha
- **Supersedes:** ADR-0001 (reuse vala-panel-appmenu-registrar)

## Context

ADR-0001 (2026-05-04) decided to depend on
`vala-panel-appmenu-daemon` as the well-known
`com.canonical.AppMenu.Registrar` service owner. The bridge was a
CLIENT of that daemon: subscribed to `WindowRegistered` /
`WindowUnregistered` signals, resolved per-PID menu paths via
`org.freedesktop.DBus.GetConnectionUnixProcessID`.

Two facts surfaced over the v0.1 ship cycle:

1. `vala-panel-appmenu` is **not packaged in nixpkgs**. PR #374302
   (the only package proposal) was closed unmerged after the
   upstream maintainer abandoned the project. There is no functional
   alternative shipping a `com.canonical.AppMenu.Registrar` daemon
   on a vanilla NixOS install.
2. ADR-0001 itself flagged this risk: "reconsidered in v0.2 if
   `vala-panel-appmenu` becomes unmaintained." That condition is
   met.

Net effect during v0.1 deployment: no registrar service runs,
Qt/GTK apps that try to call `RegisterWindow` get
`ServiceUnknown`, silently give up, and Pedro's bar can never show
their menus regardless of the bridge's correctness on every other
axis. The v0.1 work was strictly necessary scaffolding but cannot
deliver the headline feature without a registrar.

## Decision

The bridge BECOMES the registrar. It owns the well-known name
`com.canonical.AppMenu.Registrar` on the user session bus and
implements the canonical interface server-side.

Implementation (`bridge/src/registrar.rs`):
- `AppMenuRegistrar` struct exporting the canonical interface
  methods: `RegisterWindow(xid, menu_path)`,
  `UnregisterWindow(xid)`, `GetMenuForWindow(xid)`.
- Internal state: `xid → (busName, menuPath)` map (canonical
  D-Bus-spec contract) plus parallel `xid → pid` index for
  cleanup-by-xid without re-resolving a (possibly-gone)
  connection's PID.
- Echoes the canonical `WindowRegistered` / `WindowUnregistered`
  signals so other consumers (KDE Plasma's appmenu applet, future
  Quickshell native consumer) can subscribe normally.
- Publishes a `pid → (busName, menuPath)` `MenuMap` on every
  registration change, exactly the same shape `active.rs` consumed
  in v0.1 — no downstream module changes.

## Consequences

- **Apps register against us, not vala-panel-appmenu.** Qt5 apps
  with `QT_QPA_PLATFORMTHEME=appmenu-qt5` and GTK apps with
  `GTK_MODULES=appmenu-gtk-module` now have a registrar to talk to.
  Setting those env vars (NixOS module side, future commit) is the
  remaining wiring for full Anki coverage.
- **Apps that natively export DBusMenu without env vars work
  immediately.** GIMP, Inkscape, qbittorrent, KeePassXC, Audacious,
  Okular all do. v0.2.0-alpha makes those visible to the bridge
  without further configuration.
- **Name collision behaviour: safe-fail.** If another registrar
  daemon already owns the name (user has KDE Plasma running, or a
  legacy vala-panel-appmenu install), our `request_name` call
  fails and the bridge logs a warning. The registrar code exits;
  rest of the bridge (focus + proxy + active.json) keeps running
  with the v0.1 fallback. No clobbering. Future hardening could
  add `RequestNameFlags::REPLACE_EXISTING` behind a config flag.
- **Eliminates the vala-panel-appmenu vendor dependency.**
  noctalia-appmenu now ships a complete registrar in-process; no
  separate daemon, no extra Nix derivation to maintain.
- **`MenuMap` shape unchanged.** Downstream `active.rs` consumes
  the same `pid → (busName, menuPath)` map shape from a
  `watch::Sender` — no API drift, no migration. Internal
  representation switched from "events from a remote registrar"
  to "events from our interface methods".

## Wayland xid handling

DBusMenu's `RegisterWindow(xid, path)` was designed for X11 where
xids uniquely identify windows. On Wayland (including Xwayland-
hosted apps like Anki on Pedro's setup):

- Xwayland apps report a real X11 xid → works as designed.
- Native Wayland Qt6 apps report a synthetic id derived from
  `wl_surface` (Qt6's QPlatformWindow::winId()).
- Some apps report 0 (Firefox under default config).

We treat the xid opaquely as an opaque key — never look up the X
server to validate it. The pid lookup via
`org.freedesktop.DBus.GetConnectionUnixProcessID` is the
authoritative cross-reference for `active.rs`'s focus subsystem
(which already keys on PID, not xid).

## Alternatives considered

- **Vendor `vala-panel-appmenu` as a Nix derivation:**
  ~30-60 min of packaging work + ongoing maintenance burden as the
  upstream is unmaintained. Rejected for two reasons:
  (a) Pedro would need a separate user service to run alongside the
  bridge, doubling the deploy surface.
  (b) The valapanel registrar has its own bug history we'd inherit
  (XID-collision crashes, slow startup, no IPv6).
- **Pull `appmenu-registrar` (Plasma's standalone) from
  `kdePackages`:** doesn't exist as a separate package; bundled
  into KDE Plasma's plasma-workspace — installing that pulls in
  every plasmoid, kwin, plasmashell. Not appropriate for a niri
  user.
- **Run the bridge as a systemd-user oneshot for registration only,
  then exit:** loses the focus subsystem and proxy. The registrar
  must outlive every focus event — has to be the same long-running
  process as the rest of the bridge.

## References

- `com.canonical.dbusmenu` spec:
  https://github.com/AyatanaIndicators/libdbusmenu/blob/master/libdbusmenu-glib/com.canonical.dbusmenu.xml
- ADR-0001 (superseded by this).
- Spec 002 (DBusMenu mirror) — Phase A delivered here; Phase B
  (menu-tree fetch) and C (widget render) follow.
- v0.2.0-alpha release notes.
