# ADR-0008 — `PopupWindow` for submenu rendering

Status: Accepted
Date: 2026-05-04

## Context

A menubar's submenus must pop out *above* the bar (z-order), follow the click position, dismiss on outside-click, and survive across multiple monitors. Two QML rendering paths:

A. `Quickshell.PopupWindow` (a sub-window with `wlr-layer-shell` semantics, anchored to a parent surface, configurable z-layer).
B. `DBusMenuItem.popup()` — assume the menu item self-renders.

Reading `quickshell/src/dbus/dbusmenu/` confirms: there is no public `popup()` method on `DBusMenuItem`. Self-rendering is not part of the consumer API.

## Decision

Use `PopupWindow` from `Quickshell` for every submenu. The widget walks the `DBusMenuItem` tree and constructs nested `PopupWindow` instances as the user navigates.

## Consequences

- **Positive:** Predictable z-order (layer-shell `top`). Multi-monitor handling is the layer-shell library's problem, not ours.
- **Negative:** Slightly more QML wiring than option B would have been.
- **Mitigation:** The popup recursion is cleanly factored into a `SubmenuPopup.qml` component reused at every depth.

## Alternatives considered

- **Native QML `Menu`:** Quickshell does not vendor `QtQuick.Controls.Menu` for layer-shell surfaces; it would conflict with the bar's positioning model. Rejected.
- **Inline expand-in-bar:** Only the first submenu fits in the bar; deeper menus would be cropped. Rejected.

## References

- [Quickshell.PopupWindow](https://quickshell.org/docs/v0.3.0/types/Quickshell/PopupWindow)
