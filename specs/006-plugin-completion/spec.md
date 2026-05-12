# Specification: Plugin completion for v1.0.0

**ID:** 006-plugin-completion
**Created:** 2026-05-12
**Author:** @phsb5321
**Constitution version:** 1.0.0
**Parent spec:** [004-project-completion](../004-project-completion/spec.md) §FR-010..FR-013

## Why

The v0.3.0 plugin ships top-level menu activation only. `plugin/AppmenuPopupWindow.qml:240` carries a `// Nested submenus: TODO alpha.19+` no-op; any menu item with `hasChildren = true` is permanently unreachable. Spec 004 §Plugin (FR-010..FR-013) gates `v1.0.0` on four QML surfaces the audit (`research.md` §3) flagged as ship blockers or visible defects:

- **FR-010** — nested submenus (high; blocker for kate/dolphin/Anki menus with `Open Recent`, `Open With`, etc.).
- **FR-011** — `toggle_state` checkmark rendering (low; spec 004 §Scenario 4 — kate's `Tools → Spelling`).
- **FR-012** — `icon_name` icon rendering (low; consistency with bar theme).
- **FR-013** — multi-screen popup-routing guard (medium; user on multi-monitor host should not see the popup appear on the wrong screen).

This sub-spec lands the QML surface for those four FRs, ships a fixture-driven QML test for the new component, and inherits spec 003's fault-isolation invariants (envelope + sibling layer-shell + no `xdg_popup.grab`).

## User scenarios

### Scenario 1 — Nested submenu opens on click

**Given** kate is focused, the top-level `File` menu is open in an `AppmenuPopupWindow`
**When** the user clicks the `Open Recent` row (which has `children`)
**Then** a second-level `SubmenuPopup` opens as a sibling layer-shell `PanelWindow`, anchored to the right of the parent row (falling back to the left when the right edge would clip off-screen); the parent popup stays open; clicking a leaf in the submenu activates the AT-SPI action and closes both popups; clicking outside closes both popups.

### Scenario 2 — Checkable menu items show their state

**Given** the user opened `Tools → Spelling` and a row's `toggle_type` is `"checkmark"`
**When** the popup renders
**Then** the row shows a visible `✓` indicator when `toggle_state === true`, an empty (alignment-reserved) slot when `false`; clicking the row activates the action and the next snapshot reflects the new state.

### Scenario 3 — Icon-bearing rows render their icon

**Given** an `active.json` row has `icon_name = "document-open-recent"`
**When** the popup row delegate renders
**Then** the Qt icon-theme icon is resolved and painted to the left of the label; rows without `icon_name` render no leading space (alignment is consistent across the column).

### Scenario 4 — Multi-screen popup-routing guard

**Given** the user is on a multi-monitor host (screen A + screen B); the focused window lives on screen A; BarWidget is mounted on both screens
**When** noise or an errant signal triggers `openAt` on screen B's BarWidget while focus is on A
**Then** screen B's popup refuses to open (logs `[appmenu] cross-screen open refused …`); screen A's popup opens normally on subsequent focused-output clicks.

## Functional requirements

This sub-spec implements four FRs from spec 004 verbatim:

- **FR-010** (= spec 004 FR-010) — `plugin/SubmenuPopup.qml` exists; the `hasChildren` click in `AppmenuPopupWindow.qml:240` opens it as a sibling top-level `PanelWindow` per ADR-0008 + spec 003 FR-005..FR-007. Clicking a leaf activates AT-SPI; both popups close.
- **FR-011** (= spec 004 FR-011) — popup row delegate renders `toggle_state` checkmark (visible `✓` when `toggle_type === "checkmark" && toggle_state === true`; reserved blank when false; never-mismatches alignment).
- **FR-012** (= spec 004 FR-012) — popup row delegate renders `icon_name` via Quickshell's icon-theme resolution (`Quickshell.iconPath(name, "")`); rows without `icon_name` render no leading space.
- **FR-013** (= spec 004 FR-013) — `BarWidget` exposes a `focusedScreenName` property bound to `Quickshell.Wayland.ToplevelManager.activeToplevel?.screens[0]?.name ?? ""`; `AppmenuPopupWindow.openAt` and `SubmenuPopup.open` refuse to open when `focusedScreenName !== ""` and `focusedScreenName !== screen.name`.

### Fault-isolation invariants (inherited from spec 003)

- `SubmenuPopup` is a **sibling top-level** layer-shell `PanelWindow` (FR-005..FR-007 of spec 003), NOT a nested `Popup` inside `AppmenuPopupWindow`.
- `WlrLayershell.keyboardFocus: WlrKeyboardFocus.None` on every popup surface (no keyboard grab; v2 deferral per ADR-0010).
- `WlrLayershell.exclusionMode: ExclusionMode.Ignore` (no exclusive zone reservation).
- Outside-click dismisses via a full-screen `MouseArea`, NOT via `xdg_popup.grab(wl_seat)` (FR-006 of spec 003).
- Every public-callable entry on `SubmenuPopup` (`open`, `close`, the row delegate's `onClicked`) is wrapped in a `try { ... } catch (e) { console.error("[appmenu/submenu] envelope caught:", e); root._failedState = true; }` envelope (FR-008 of spec 003).
- When `_failedState` is true, the popup closes and refuses to re-open until the parent applies a fresh well-formed snapshot.

## Non-functional requirements

- **NFR-001 Performance** — submenu open → render ≤ 100 ms p95 (per spec 004 NFR-001); animation deferred to v1.x polish.
- **NFR-002 Theme tokens** — `Color.m*` + `Style.*` only; zero raw hex / rgb / Tailwind-style arbitrary spacing (constitution + ADR-0018).
- **NFR-003 Verifiability** — every FR has a QML test fixture or a runtime check (`console.log` trace for FR-013 refusal).

## Out of scope

- Keyboard navigation, mnemonic underlines, Alt-F intercept (deferred to v2 per ADR-0010).
- Cascading hover (auto-open submenu on hover after a delay); v1 is click-to-open only.
- Animated open/close transitions; v1 is instant. Animation is a v1.x polish task.
- Icon theme tinting / colour-overlay to match `Color.mOnSurface` exactly; v1 renders the icon as the theme delivers it. Polish in v1.x.
- Lane A's `focused_output` field in `active.json`; if Lane A ships it, `BarWidget` MAY prefer it over `Quickshell.Wayland.ToplevelManager` — but the FR-013 guard works with either source.

## Constraints / dependencies

- Quickshell ≥ 0.3.0 (`Quickshell.iconPath`, `Quickshell.Wayland.ToplevelManager`).
- noctalia-shell ≥ 1.0.0 (v4 single-PanelWindow assumption stays valid per spec 003).
- The existing `MenuItem` shape in `active.json` (per spec 004 [`contracts/active-json-schema.md`](../004-project-completion/contracts/active-json-schema.md)) is the input contract; this spec adds nothing to the schema.

## Success criteria

- **SC-001** `qmllint plugin/BarWidget.qml plugin/AppmenuPopupWindow.qml plugin/SubmenuPopup.qml plugin/MenuRow.qml` exits clean (pre-existing unqualified-access advisories preserved; no new errors).
- **SC-002** `nix flake check` passes on the new branch (no Rust regression).
- **SC-003** `plugin/tests/qmltest/submenu_popup.qml` instantiates the new component against a hand-crafted JSON tree and asserts: nested popup opens on `hasChildren` click, `toggle_state` indicator visible when checked, icon renders when `icon_name` is set, cross-screen open is refused.
- **SC-004** Manual smoke against the fixture confirms scenarios 1–4 above.

## Key entities

- **`SubmenuPopup` (new)** — `plugin/SubmenuPopup.qml`. Sibling layer-shell `PanelWindow` per ADR-0008. Instantiated by `AppmenuPopupWindow` (depth 2) and recursively by itself (depth ≥ 3) via a local `Component { SubmenuPopup { } }`.
- **`MenuRow` (new, optional)** — `plugin/MenuRow.qml`. Shared row delegate; renders `label`, `enabled`, `toggle_state` indicator (FR-011), `icon_name` icon (FR-012), submenu `›` chevron; emits `clicked(item)` and `submenuRequested(item, anchorRect)`.
- **`AppmenuPopupWindow` (modified)** — instantiates `MenuRow` in its `Repeater`; the `hasChildren` click opens a child `SubmenuPopup` (FR-010 wiring).
- **`BarWidget` (modified)** — exposes `focusedScreenName` (bound to `ToplevelManager.activeToplevel`); passes it to the popup.

## Assumptions

- `Quickshell.iconPath(name, fallback)` is available in v0.3.0 and returns either a URL string (`image://icon/<name>`) or the `fallback` string when the icon is missing. The Quickshell upstream documentation lists it on the `Quickshell` singleton.
- `Quickshell.Wayland.ToplevelManager.activeToplevel` is non-null when a Wayland toplevel has focus; its `screens` property is a list of `ShellScreen` objects with a `name` field. When the property is null/empty, `focusedScreenName` is `""` and the FR-013 guard is permissive (does not refuse the open).
- Recursive QML self-reference via `Component { SubmenuPopup { } }` declared inside `SubmenuPopup.qml` resolves at instantiation time, not at component-graph-build time, so it terminates correctly on real (finite-depth) menu trees.
