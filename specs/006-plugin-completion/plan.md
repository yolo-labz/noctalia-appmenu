# Implementation plan: plugin completion (v0.3.0 → v1.0.0)

**Spec:** `specs/006-plugin-completion/spec.md`
**Parent:** `specs/004-project-completion/{spec,plan}.md`
**Constitution version:** 1.0.0
**Generated:** 2026-05-12

## Approach

Four FRs from spec 004 (FR-010 nested submenus, FR-011 toggle_state, FR-012 icon_name, FR-013 multi-screen guard) land in one branch `75-plugin-completion`. Lane B per spec 004 §Rollout §3 — branches off `origin/main` (NOT `004-project-completion`); inherits spec 003 fault-isolation invariants verbatim.

The implementation strategy is conservative: add the missing `SubmenuPopup.qml` component as a sibling layer-shell `PanelWindow` per ADR-0008 + spec 003 FR-005..FR-007; factor a shared `MenuRow.qml` so `AppmenuPopupWindow` and `SubmenuPopup` render identically (DRY + single point of evolution for FR-011/FR-012); thread a `focusedScreenName` property from `BarWidget` through the popup tree as the FR-013 guard source. No bridge (Rust) work; no Nix / CI / docs work — those land in Lanes A / C / D.

Self-referencing recursive component pattern: `SubmenuPopup.qml` declares a local `Component { SubmenuPopup { } }` for the deeper-level instance. QML parses the inner type as a deferred component, so the recursion terminates on the finite-depth AT-SPI menu tree (typically ≤ 4 levels per spec 004 contract). Each level is its own top-level `PanelWindow` — no `Popup` nesting, no `xdg_popup.grab`.

Multi-screen guard is implemented as a property-driven refusal: `BarWidget.focusedScreenName` is bound to `Quickshell.Wayland.ToplevelManager.activeToplevel?.screens[0]?.name ?? ""`. `AppmenuPopupWindow.openAt` and `SubmenuPopup.open` consult `focusedScreenName` — when non-empty and ≠ `screen.name`, they short-circuit and log. When empty (single-screen host or no toplevel-tracking available), the guard is permissive. Lane A may later supply a `focused_output` field in `active.json`; the property hook accepts either source.

## Constitution check

| Principle | Status | Notes |
|---|---|---|
| I — niri-only v1 | PASS | No compositor-agnostic code; multi-screen guard uses Wayland-generic `ToplevelManager`. |
| II — Sidecar by default | PASS | No bus-name acquisition in QML; the plugin still consumes `active.json` + IPC push only. |
| III — Worktree-first git | PASS | Branch `75-plugin-completion` off `origin/main` in `../noctalia-appmenu-75-plugin-completion/`. |
| IV — Conventional Commits + DCO | PASS | All commits `git commit -s -m "feat(plugin): ..."` etc. |
| V — Speckit-driven | PASS | Spec + plan + tasks + checklist before code. |
| VI — Release-engineering compliance | PASS | No workflow / SBOM changes; Lane D owns those. |
| VII — Graceful degradation | PASS | FR-013 guard is permissive when `focusedScreenName` is empty; falls back to current behaviour. |

## Architecture sketch

```
BarWidget (per-screen)
 ├── focusedScreenName ← ToplevelManager.activeToplevel?.screens[0]?.name
 ├── Row { Repeater { delegate: top-level button } }
 └── AppmenuPopupWindow              ← sibling layer-shell PanelWindow (depth 1)
       ├── Rectangle { menuBox }
       ├── Repeater { delegate: MenuRow }
       │     ↑ click on hasChildren → openAt of …
       └── SubmenuPopup               ← sibling layer-shell PanelWindow (depth 2)
             ├── Rectangle { menuBox }
             ├── Repeater { delegate: MenuRow }
             └── Loader { sourceComponent: nestedComponent }  ← deferred
                   nestedComponent: Component { SubmenuPopup { } }  ← depth ≥ 3
```

Each `PanelWindow` is independent — Wayland routes input surface-by-surface; the bar stays clickable; outside-click closes via full-screen `MouseArea`; no `xdg_popup.grab`.

## Affected files

- `plugin/SubmenuPopup.qml` (new — recursive sibling layer-shell popup)
- `plugin/MenuRow.qml` (new — shared row delegate; FR-011 + FR-012 rendering)
- `plugin/AppmenuPopupWindow.qml` (modified — use `MenuRow` in `Repeater`; wire `hasChildren` click → `SubmenuPopup.open`; add `focusedScreenName` property; add guard in `openAt`)
- `plugin/BarWidget.qml` (modified — `focusedScreenName` property + `ToplevelManager` binding; pass through to popup)
- `plugin/tests/qmltest/submenu_popup.qml` (new — fixture test driving the component with a hand-crafted JSON tree)
- `plugin/tests/qmltest/README.md` (new — terse run instructions)
- `specs/006-plugin-completion/{spec,plan,tasks}.md` + `checklists/requirements.md` (this work)

## Risks

- **R1** Quickshell `iconPath` API name or signature may differ from `Quickshell.iconPath(name, fallback)` across v0.3.x. *Mitigation*: wrap the call in a try/catch + fallback to `"image://icon/" + name` URL scheme; if neither resolves, the icon is invisible (no row breakage).
- **R2** `Quickshell.Wayland.ToplevelManager.activeToplevel.screens` may be empty during the brief window between focus changes. *Mitigation*: the guard explicitly tolerates empty `focusedScreenName` — refuses ONLY when non-empty AND mismatched.
- **R3** Recursive `Component { SubmenuPopup { } }` may produce a qmllint warning about self-reference. *Mitigation*: the pattern is documented Qt practice; if qmllint flags it, the warning is advisory (no runtime effect) and is added to the baseline.
- **R4** Lane B's fixture cannot exercise the full multi-screen path without a real second output. *Mitigation*: the fixture drives `focusedScreenName` manually (via a property setter on the harness), asserting the refusal log path fires; real multi-screen verification happens manually post-merge per SC-004.

## Rollout

1. Land sub-spec + plan + tasks + checklist (this PR).
2. Land `MenuRow.qml` + `AppmenuPopupWindow.qml` refactor + `BarWidget.qml` guard wiring (one commit).
3. Land `SubmenuPopup.qml` + nested-submenu wiring (one commit).
4. Land `tests/qmltest/submenu_popup.qml` fixture + README (one commit).
5. Push branch `75-plugin-completion`; report to parent for PR.

## Open questions

1. Should `MenuRow.qml` also be used by `BarWidget.qml`'s top-level strip (currently inlined as `Rectangle { btn }`)? *Default*: no — the strip's rendering is meaningfully different (horizontal vs. vertical, no `›` chevron, no toggle indicator). Refactor deferred to a v1.x polish task.
2. Should the FR-013 guard `console.log` use `console.error`? *Default*: `console.log` — the refusal is expected behaviour, not an error; spec 003 NFR-003 only requires error logging on isolation envelope catches.
