# DBusMenu pipeline

The full life of a menu item from app to bar.

## At app startup

1. The Qt or GTK app loads its appmenu module. The module installs an internal hook on the app's main `QMenuBar` / `GtkMenuShell`.
2. The hook serialises the menu tree into a `com.canonical.dbusmenu` interface mounted at a per-app object path on the app's own bus connection.
3. The hook calls `com.canonical.AppMenu.Registrar.RegisterWindow(windowId, menuObjectPath)` on the standalone registrar daemon.
4. The registrar emits `WindowRegistered(windowId, busName, menuObjectPath)`.

## In the bridge

5. `bridge::registrar` receives `WindowRegistered`, ignores the `windowId` (X11-only), reads the *signal sender's* bus name from the message header, and calls `org.freedesktop.DBus.GetConnectionUnixProcessID(sender)` to get the registering process's PID.
6. The bridge stores `pid → (busName, menuObjectPath)` in `MenuMap.by_pid`.

Meanwhile:

7. `bridge::niri` receives `WindowFocusChanged{id}` from `niri msg --json event-stream`.
8. It looks up `id → pid` from a cached snapshot of `niri msg --json windows`.
9. It emits `FocusEvent { winid, pid, app_id, title }` onto the focus channel.

## In the joiner

10. `bridge::active` listens on both watch channels (`focus_rx`, `menus_rx`).
11. After 75 ms of stillness on either input, it calls `active::snapshot(focus, menus)`:
    - If no focus → empty snapshot.
    - If focus + matching menu → full snapshot (bus, path, app_id, title).
    - If focus + no matching menu → snapshot with empty bus/path but valid app_id (downstream renders fallback pseudo-menu).
12. The joiner publishes the snapshot on the active channel.

## In the proxy

13. `bridge::proxy` owns `org.noctalia.AppMenu` and exposes `/org/noctalia/AppMenu/Active` with four properties: `busName`, `objectPath`, `appId`, `title`.
14. On every active-snapshot change, the proxy updates its inner state and emits `<prop>_changed` for each of the four properties.

## In the QML widget

15. `BarWidget.qml` binds a `DBusObject` to `org.noctalia.AppMenu /org/noctalia/AppMenu/Active`.
16. As `busName` and `objectPath` change, the widget rebinds a `Quickshell.DBusMenu.DBusMenuHandle` to the new `(bus, path)`.
17. The handle's `menu` property is a root `DBusMenuItem`; the widget renders its `children` as a horizontal row of `MenuButton`s.
18. Click → open `SubmenuPopup` → recursive render → click leaf → `DBusMenuItem::activate()` → app handles the menu action.

## Wire-level summary

- App side (out-of-tree, ours to NOT touch): `appmenu-qt5` / `appmenu-gtk-module-wayland`.
- Registrar side (out-of-tree, runtime dep): `vala-panel-appmenu`'s `appmenu-registrar`.
- Our side: `bridge` (Rust) + `BarWidget.qml`.
