# Requirements checklist: plugin completion (spec 006)

**Parent FRs:** spec 004 FR-010 (nested submenus), FR-011 (toggle_state), FR-012 (icon_name), FR-013 (multi-screen guard).

## FR-010 — SubmenuPopup component

- [ ] `plugin/SubmenuPopup.qml` exists at the worktree root's plugin dir.
- [ ] `SubmenuPopup` is a top-level `PanelWindow`, NOT a `Popup` nested inside `AppmenuPopupWindow` (spec 003 FR-005..FR-007).
- [ ] `WlrLayershell.layer: WlrLayer.Top`, `keyboardFocus: WlrKeyboardFocus.None`, `exclusionMode: ExclusionMode.Ignore`, `namespace: "noctalia-appmenu-submenu-" + screen.name`.
- [ ] Outside-click closes via full-screen `MouseArea`; NO `xdg_popup.grab(wl_seat)` calls.
- [ ] `AppmenuPopupWindow.qml:240` `hasChildren` click opens the `SubmenuPopup` instead of being a no-op.
- [ ] Recursive nesting (depth ≥ 3) works via local `Component { SubmenuPopup { } }`.
- [ ] Leaf-click inside the submenu propagates to `BarWidget.fireClick` and closes the chain.
- [ ] Every entry point (`open`, `close`, delegate `onClicked`) wrapped in spec 003 try/catch envelope.

## FR-011 — toggle_state rendering

- [ ] Row delegate reserves an indicator slot when `toggle_type !== null`.
- [ ] Renders `✓` (theme-tinted) when `toggle_type === "checkmark" && toggle_state === true`.
- [ ] Renders blank-but-aligned when `toggle_state === false`.
- [ ] Rows with `toggle_type === null` render no indicator slot (no leading space change).

## FR-012 — icon_name rendering

- [ ] Row delegate resolves `icon_name` via Quickshell's icon-theme lookup (e.g. `Quickshell.iconPath(name, "")`).
- [ ] `Image` element is invisible when `icon_name` is empty (no leading space).
- [ ] `Image` element renders at a consistent size aligned with the label baseline.

## FR-013 — multi-screen popup-routing guard

- [ ] `BarWidget` exposes `focusedScreenName` bound to a source-of-truth (Quickshell ToplevelManager or active.json's `focused_output`).
- [ ] `AppmenuPopupWindow.openAt` refuses to open when `focusedScreenName !== "" && focusedScreenName !== screen.name`.
- [ ] `SubmenuPopup.open` refuses to open under the same condition.
- [ ] Refusal logs `[appmenu] cross-screen open refused …` to `journalctl --user -u noctalia-shell`.
- [ ] When `focusedScreenName` is empty (no toplevel-tracking available), the guard is permissive.

## Theme + style

- [ ] Zero raw hex / rgb / Tailwind-style arbitrary spacing in new files.
- [ ] All colours via `Color.m*` tokens; all spacing via `Style.*` tokens.
- [ ] Font size: `Style._barBaseFontSize * (Settings.data.bar.fontScale || 1.0)` (matches existing files).

## Fault isolation (spec 003)

- [ ] `SubmenuPopup` does NOT use `Popup`/`PopupWindow` with `grabFocus: true`.
- [ ] No `Timer { repeat: true }` with interval < 200 ms (spec 003 FR-011).
- [ ] `_failedState` flag wired on `SubmenuPopup`; closes + refuses on throw until fresh open.

## Tests

- [ ] `plugin/tests/qmltest/submenu_popup.qml` exists.
- [ ] Test instantiates the component against a hand-crafted JSON tree.
- [ ] Test asserts: nested popup opens, `toggle_state` indicator visible, icon renders, cross-screen open refused.
- [ ] `plugin/tests/qmltest/README.md` documents the run command.

## CI gates

- [ ] `qmllint plugin/BarWidget.qml plugin/AppmenuPopupWindow.qml plugin/SubmenuPopup.qml plugin/MenuRow.qml` clean (no new errors).
- [ ] `nix flake check` passes (no Rust regression).
- [ ] All commits DCO-signed (`git commit -s`).
- [ ] Branch pushed; no PR opened by the worker (parent opens it).
