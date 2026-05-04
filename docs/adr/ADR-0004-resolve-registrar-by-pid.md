# ADR-0004 — Resolve registrar entries by D-Bus connection PID

Status: Accepted
Date: 2026-05-04

## Context

`com.canonical.AppMenu.Registrar.RegisterWindow(uint32 windowId, ObjectPath menuObjectPath)` was designed in the X11 era. `windowId` is documented as type `u` (uint32) — meaning an X11 XID. On Wayland, there is no canonical XID. Some clients send `0`; some send a synthetic counter; some apps that run XWayland send the real XID.

Plasma sidesteps this entirely on Wayland: KWin couples its window-management protocol with appmenu, so each `PlasmaWindow` carries `applicationMenuObjectPath` + `applicationMenuServiceName` directly. We don't have that on niri.

So we cannot trust `windowId`. We must resolve registrar entries to a process some other way.

## Decision

When the registrar emits `WindowRegistered(windowId, busName, objectPath)`, the bridge calls `org.freedesktop.DBus.GetConnectionUnixProcessID(busName)` to obtain the registering process's PID. The bridge maintains `menu_by_pid: HashMap<u32, (busName, objectPath)>` and ignores `windowId` entirely.

Focus side: the bridge subscribes to niri-IPC's `WindowFocusChanged{id}` events, looks up `pid_by_winid[id]`, finds the matching `menu_by_pid` entry, and re-publishes that menu under the active proxy.

## Consequences

- **Positive:** Compositor-independent (the same logic works on any Wayland compositor that gives us focused-window-pid; niri does, others can be added in v2). Robust against `windowId=0` and synthetic-id schemes.
- **Negative:** Apps that fork and have their *menu* registered from a child process (rare — Chromium's possibility) will pubsub off the registering child's PID, not the visible-window's PID. We document this and provide a debug knob.
- **Mitigation:** Verified pattern with Chromium's DBus appmenu code that the *registering* connection's PID is the main browser process's PID, not a renderer child. Document one-liner systemd-cgls debug recipe in `CLAUDE.md`.

## Alternatives considered

- **Trust `windowId`:** Useless on Wayland. Rejected.
- **`appId+title` heuristic match against the niri window list:** Collides on multiple windows of the same app. Rejected (also see ADR-0002).
- **`org_kde_kwin_appmenu` Wayland protocol:** Implemented only by KWin; niri does not advertise it. Rejected for v1.

## References

- [Kai Uwe Broulik — On Window Activation (2025)](https://blog.broulik.de/2025/08/on-window-activation/)
- `applet-window-appmenu/plugin/wm/waylandwindowmanager.cpp:117-148` — Plasma's reference impl
