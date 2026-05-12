# Tasks: plugin completion

**Spec:** `specs/006-plugin-completion/spec.md`
**Plan:** `specs/006-plugin-completion/plan.md`
**Budget:** ≤ 25 tasks (constitution Development Workflow §3).

| # | Task | FR | Outcome |
|---|---|---|---|
| 1 | Sub-spec scaffolding: `spec.md`, `plan.md`, `tasks.md`, `checklists/requirements.md`. | — | Spec chain in place. |
| 2 | Create `plugin/MenuRow.qml` — shared row delegate; renders `label`, `enabled`, `›` chevron when `hasChildren`. | FR-010 | Shared component compiles. |
| 3 | Extend `MenuRow.qml` with `toggle_state` indicator slot (`✓` when `toggle_type === "checkmark" && toggle_state === true`; reserved blank when `false`; absent when `toggle_type === null`). | FR-011 | Checkmark renders; alignment preserved. |
| 4 | Extend `MenuRow.qml` with `icon_name` icon slot (Quickshell `iconPath` resolution; blank when empty). | FR-012 | Icon renders when present. |
| 5 | Emit `clicked(item)` (leaf) and `submenuRequested(item, anchorRect)` (parent of children) signals from `MenuRow`. | FR-010 | Row drives parent popup. |
| 6 | Wrap `MenuRow.onClicked` in spec 003 try/catch envelope (`console.error("[appmenu/row] envelope caught:", e)`). | FR-010 (FR-008 spec 003) | Single broken delegate cannot poison siblings. |
| 7 | Refactor `AppmenuPopupWindow.qml`'s inline `Repeater { delegate: Item { … } }` to `Repeater { delegate: MenuRow { } }`. | FR-010..FR-012 | DRY; identical surface. |
| 8 | Add `focusedScreenName: string` property to `AppmenuPopupWindow.qml`. | FR-013 | Guard plumbing in place. |
| 9 | Guard `AppmenuPopupWindow.openAt` — refuse when `focusedScreenName !== "" && focusedScreenName !== screen.name`; log `[appmenu] cross-screen open refused …`. | FR-013 | Cross-screen open refused. |
| 10 | Create `plugin/SubmenuPopup.qml` — sibling layer-shell `PanelWindow` (`WlrLayer.Top`, `keyboardFocus: None`, `exclusionMode: Ignore`, `namespace: "noctalia-appmenu-submenu-" + screen.name`). | FR-010 (FR-005..FR-007 spec 003) | Sibling top-level surface; no `Popup` nesting. |
| 11 | Add `SubmenuPopup` properties: `screen`, `parentItem`, `parentMenuItem`, `anchorRect`, `focusedScreenName`. | FR-010 + FR-013 | Component shape per contract. |
| 12 | Implement `SubmenuPopup.open(item, anchorRect)` — sets state, computes anchor (right edge of parent rect, falling back to left when clipped), shows. | FR-010 | Submenu opens at correct anchor. |
| 13 | Implement `SubmenuPopup.close()` — hides; clears `_failedState` not required (drained on next open). | FR-010 | Submenu closes. |
| 14 | Guard `SubmenuPopup.open` — refuse when `focusedScreenName !== "" && focusedScreenName !== screen.name`. | FR-013 | Cross-screen open refused at depth ≥ 2 too. |
| 15 | `SubmenuPopup` outside-click → close (full-screen `MouseArea`, NOT `xdg_popup.grab`). | FR-010 (FR-006 spec 003) | Outside-click dismisses. |
| 16 | `SubmenuPopup` `Repeater { delegate: MenuRow }` — same delegate as `AppmenuPopupWindow`. | FR-010..FR-012 | Symmetric rendering. |
| 17 | `SubmenuPopup` recursive nesting — declare local `Component { SubmenuPopup { } }`; `Loader` source-bound on `submenuRequested` from row delegate. | FR-010 | Depth ≥ 3 supported. |
| 18 | `SubmenuPopup` envelope on `open` + `close` + delegate signals (spec 003 FR-008). Sets `_failedState` on throw; refuses re-open until fresh `open` call. | FR-010 (FR-008 spec 003) | Fault-isolated. |
| 19 | Wire `AppmenuPopupWindow.qml:240` — on `hasChildren` click, instantiate / `open` the local `SubmenuPopup`; signal upward when a deep leaf is activated so `BarWidget.fireClick` runs. | FR-010 | Nested submenu chain works end-to-end. |
| 20 | Add `focusedScreenName` to `BarWidget.qml`; bind to `Quickshell.Wayland.ToplevelManager.activeToplevel?.screens?.[0]?.name ?? ""` via a defensive expression. | FR-013 | Source-of-truth wired. |
| 21 | Pass `focusedScreenName` from `BarWidget` → `AppmenuPopupWindow` → `SubmenuPopup`. | FR-013 | Guard data flows down. |
| 22 | Create `plugin/tests/qmltest/submenu_popup.qml` — `QtTest TestCase` instantiating the component with a hand-crafted JSON tree (one parent with `children`, one leaf with `toggle_state`, one leaf with `icon_name`); asserts render + open + refusal. | FR-010..FR-013 | Fixture coverage. |
| 23 | Add `plugin/tests/qmltest/README.md` — `nix develop --command qmltestrunner -input plugin/tests/qmltest`. | — | Reproducible test recipe. |
| 24 | Verify `qmllint plugin/BarWidget.qml plugin/AppmenuPopupWindow.qml plugin/SubmenuPopup.qml plugin/MenuRow.qml` exits clean (pre-existing advisories only). | SC-001 | Lint gate green. |
| 25 | Verify `nix flake check` passes (no Rust regression). | SC-002 | Build/test gate green. |

Acceptance per spec 006 §SC-001..SC-004; each task DCO-signed; conventional-commit subject.
