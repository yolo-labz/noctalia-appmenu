# Contract: `active.json` schema (additive)

Spec: `specs/009-popup-hotfix/spec.md` FR-006

## Producer

`bridge/src/active.rs` writes the snapshot. After this hotfix, the
written object MAY include an additional `focused_output` field.

## Consumer

`plugin/BarWidget.qml` reads the snapshot via `IpcHandler.update`
(steady state) or `FileView.onLoaded` (cold start). Both paths
parse the same JSON.

## Schema (after hotfix)

```jsonc
{
  "v": 1,                                      // unchanged
  "app_id": "com.example.App",                 // unchanged
  "focus_pid": 12345,                          // unchanged
  "title": "Window title",                     // unchanged
  "menu_service": "org.example.AppMenu",       // unchanged
  "menu_path": "/org/example/AppMenu/menubar", // unchanged
  "menu": null | { ... },                      // unchanged
  "focused_output": "DP-1" | null              // NEW (optional)
}
```

`focused_output`:
- TYPE: `string | null`
- PRESENCE: optional (consumers MUST tolerate absence)
- VALUES: a wayland output name (matches one of `Screen.name` from
  `Quickshell.screens`) when known, `null` when bridge cannot
  determine the focused output (rare — compositor restart, focus
  on an unrouted toplevel)

## Test contracts

- **Producer-side.** `bridge/tests/active_json.rs` (new or extend
  existing) asserts:
  - With a niri focus event for a window on `DP-1`,
    `serde_json::to_string(&snapshot)` contains
    `"focused_output":"DP-1"`.
  - With no focused window (compositor restart), the serialised
    JSON contains `"focused_output":null`.
  - The order of keys does NOT matter (consumers parse, not
    string-match).
- **Consumer-side.** `plugin/tests/qmltest/popup_geometry.qml`
  (Lane Q test, see below) constructs a fake snapshot with
  `focused_output: "FAKE-1"`, calls `applySnapshot`, asserts
  `focusedScreenName === "FAKE-1"` after one tick.
- **Backward compatibility.** Feed a v1.0.0 snapshot (no
  `focused_output` field) into the QML widget; assert the widget
  still renders normally and `focusedScreenName` falls through to
  the existing Quickshell-then-empty path.

## Versioning

`v` stays at `1`. Bumping `v` is reserved for breaking changes;
this addition is forward-compatible.
