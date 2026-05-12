# `plugin/tests/qmltest/` — QML fixture tests

QtTest fixtures for the `plugin/` QML surface (spec 006).

## Run

```sh
nix develop --command qmltestrunner -input plugin/tests/qmltest
```

Runner ships with Qt 6 `qttools` (already in `flake.nix` devShell).

## Coverage scope

`submenu_popup.qml` exercises the rendering + signal surface of
`MenuRow.qml` against a hand-crafted JSON tree shaped per spec 004
[`contracts/active-json-schema.md`](../../../specs/004-project-completion/contracts/active-json-schema.md).
It asserts:

- **FR-010** — rows with `children` emit `submenuRequested`, leaves
  emit `clicked`.
- **FR-011** — `toggle_state` indicator renders `✓` when on, blank
  (slot reserved) when off; absent when `toggle_type === null`.
- **FR-012** — `icon_name` resolves to `image://icon/<name>` via the
  Quickshell `iconPath` API contract; empty `icon_name` falls back to
  empty string (Image is not rendered).
- **FR-013** — multi-screen guard refuses when popup-screen ≠
  focused-screen; permits same-screen; permissive when focused-screen
  unknown.

## Out of scope under qmltestrunner

`plugin/SubmenuPopup.qml` and `plugin/AppmenuPopupWindow.qml` are
top-level layer-shell `PanelWindow`s. Layer-shell needs a
wlr-layer-shell-capable compositor (niri / sway / Hyprland); under
qmltestrunner there is no compositor, so instantiating those types
crashes. Their integration is verified end-to-end by:

1. Lane D's AT-SPI integration test (spec 004 FR-022; runs on
   `vm103` with a headless niri).
2. Manual smoke per spec 006 SC-004 — open kate, click `File`, hover
   `Open Recent`, verify the submenu appears as a sibling top-level
   surface and clicking a leaf closes the chain.

The MenuRow-level assertions plus the pure-JS guard branch cover the
testable surface offered by the headless harness.
