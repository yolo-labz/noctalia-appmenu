# Specification: v1.0.0 popup hotfix — submenu cascade + render fidelity

**ID:** 009-popup-hotfix
**Created:** 2026-05-15
**Author:** @phsb5321
**Constitution version:** 1.0.0

## Why

v1.0.0 shipped on 13/05/2026 (commit `354edd0`, tag `v1.0.0`). On Pedro's
desktop (niri/Wayland, AMD GPU, Catppuccin Mocha) the widget renders
visibly broken when interacting with `shadPS4QtLauncher` (Qt6, native
AT-SPI exporter): top-level menus open but their dropdown is clamped to
~180px regardless of label width, submenu cascade beyond depth 1 does
not open at all, and clicking back toward the bar to switch menus
silently dismisses the popup instead of reopening on the new top-level
button. The bar feels "frozen" because click events for the bar's own
buttons are absorbed by a full-screen `MouseArea` inside the popup
window. None of these regressions appeared in the bridge journal or in
CI — they are all interaction- and geometry-level defects that only
surface against a real Qt menu tree.

Four parallel investigation agents (qml-architect,
dbusmenu-protocol-expert, niri-wayland-tester, Explore) converged on
seven defects, four of them CONFIRMED by direct code reading. Three are
in the QML widget tree, one is in the Rust bridge's AT-SPI walker, and
two are degraded behaviours that compound the others. Until these are
fixed, v1.0.0 is functionally a beta on every Qt6 app whose menu nests
deeper than two levels — which is most of them. shadPS4QtLauncher is
representative; kate, dolphin, krita, qbittorrent, lutris, all behave
the same way. The fix is mechanical, well-bounded, and unblocks the
v1.0.x patch lane without re-opening any of the architectural
decisions in ADR-0008 / 0009 / 0010 / 0024.

## User scenarios

### Scenario 1: Open a top-level menu against a Qt6 app

**Given** `shadPS4QtLauncher` (or any Qt6 app exporting AT-SPI) is the
focused window and its menubar contains items wider than 180px
("Show Labels Under Icons").
**When** the user left-clicks the `View` button on the noctalia bar.
**Then** the dropdown opens with width sized to the widest label
(plus margins), every row is fully visible (no horizontal clipping),
and the menuBox renders with a visible border and rounded corner
distinct from the bar background.

### Scenario 2: Cascade into a submenu (depth ≥ 2)

**Given** the `View` dropdown is open and contains an item with its
own children (`Game List Mode`, `Game List Icons`, `Themes`).
**When** the user clicks (or hovers, per current UX) the parent row.
**Then** the nested submenu opens immediately on a separate
layer-shell surface, anchored to the right edge of the parent row,
populated with the actual leaf labels (`List`, `Grid`, `Flat`, etc.) —
NOT a single blank row, NOT silently nothing.

### Scenario 3: Switch top-level menu while a popup is open

**Given** the `View` dropdown is open.
**When** the user moves the cursor back over the bar and clicks the
`Settings` top-level button.
**Then** the open popup closes, the new `Settings` popup opens at the
new anchor, and the click event reaches the bar (not absorbed by an
outside-click MouseArea on the popup surface).

### Scenario 4: Re-focus the same app after a stale-menu refresh

**Given** the bridge has emitted `MenuError::Stale` and re-walked the
AT-SPI tree, producing a snapshot whose top-level labels are unchanged
but whose nested children are updated.
**When** the user opens a top-level menu after the re-walk.
**Then** the popup renders the new children, not the cached pre-stale
children — i.e. the QML model assignment is not skipped purely on
top-level identity.

### Scenario 5: Bar switches focused app

**Given** focus moves from `shadPS4QtLauncher` to `ghostty` (terminal,
no AT-SPI menu).
**When** the bridge writes `menu: null` to `active.json`.
**Then** the bar widget collapses to its zero-paint stable slot
(or the configured `fallbackText`), without a visible flicker, and
without latching `_failedState = true` permanently.

## Functional requirements

- **FR-001 Recursive Qt wrapper-flatten in AT-SPI walker.** The bridge
  MUST strip `MENU_ITEM → MENU(empty label) → [items]` wrapper shapes
  at every level of the tree, not only at the top level. After
  flatten, each menu item's `children` field is its real action list
  (no unnamed `MENU` sentinel intermediates). Test contract: feed a
  fixture mirroring `shadPS4QtLauncher`'s `View > Game List Mode >
  [List, Grid, Flat]` accessibility tree; assert that
  `walk(tree).children[0].children[0].children == [List, Grid, Flat]`
  with no empty-label parent.
- **FR-002 Constrained popup surface (no full-screen MouseArea).** The
  `AppmenuPopupWindow` and `SubmenuPopup` PanelWindows MUST NOT
  anchor to the full screen. They MUST size to `menuBox.height` and
  position via explicit `y` calculated from the anchor item, so that
  cursor input over the bar does not land on the popup surface. Test
  contract: with a popup open, simulate a click in the bar's y-band on
  another top-level button — the click reaches the bar's MouseArea, not
  the popup's.
- **FR-003 `popupCol`/`submenuCol` width binding.** The menuBox width
  MUST be driven by the actual content width of the rows, not by an
  anchored Column whose `implicitWidth` is zero. Test contract: a row
  with label `"Show Labels Under Icons"` (≈210px in Inter 14px) MUST
  render fully — `menuBox.width >= 210 + 2 * marginM`.
- **FR-004 Async-safe Loader for nested submenu.** The recursive
  `nestedLoader` in `SubmenuPopup` MUST wait for `Loader.status ==
  Loader.Ready` before calling `open(item, anchor)` on the loaded
  item. Test contract: trigger a depth-3 cascade in the qmltest
  harness; assert `submenu.visible == true` after one event-loop tick
  (currently the call is silently dropped).
- **FR-005 Children-aware top-level dedup.** `_sameTopLevel` in
  `BarWidget.qml` MUST invalidate when any descendant has changed,
  OR the dedup MUST be narrowed to skip only when the bridge confirms
  the snapshot is identical (e.g. same source-event id). The current
  identity-only check causes stale children after a `MenuError::Stale`
  re-walk. Test contract: synthesise two snapshots with identical
  top-level ids/labels but different second-level children; assert
  `topLevel` is reassigned and the popup re-renders the new tree.
- **FR-006 Cross-screen guard fallback.** When
  `Quickshell.Wayland.ToplevelManager.activeToplevel.screens` is
  empty (the niri "no `enter` event yet" case), the
  `focusedScreenName` derivation MUST fall back to the bridge-provided
  `focused_output` field in `active.json` (if present) before
  defaulting to empty. Test contract: with `screens=[]` and
  `active.json.focused_output="DP-1"`, `focusedScreenName == "DP-1"`.
- **FR-007 Recursive submenu namespace uniqueness.** Each level of
  recursive `SubmenuPopup` MUST use a distinct
  `WlrLayershell.namespace` string (e.g. depth suffix). Test contract:
  introspect live wayland surfaces during a depth-3 cascade; the three
  `noctalia-appmenu-submenu-*` surfaces MUST have distinct namespace
  strings.
- **FR-008 `_failedState` self-clear on subsequent valid snapshot.**
  The QML envelope MUST clear `_failedState` on the next successfully
  applied snapshot, even if the snapshot is structurally identical to
  the one that caused the latch. Test contract: throw inside
  `_applySnapshotInner` once; push an identical snapshot; assert
  `_failedState == false` after the next tick.

## Non-functional requirements

- **NFR-001 No render regression at idle.** When the focused app has
  no menu, the bar widget MUST continue to claim the same horizontal
  slot it claims today (ADR-0019 reserveSlot invariant). Hotfix MUST
  NOT introduce a layout pass on focus change.
- **NFR-002 No new wl_seat grab.** Constraining the popup surface
  (FR-002) MUST NOT introduce `xdg_popup.grab(wl_seat)` — i.e. MUST
  NOT switch from `WlrLayershell` to `Quickshell.PopupWindow`.
  PR #52's protocol-level rationale (sibling layer-shell surfaces) is
  preserved.
- **NFR-003 Recursive flatten budget.** FR-001's recursive walk MUST
  complete inside the existing `FETCH_BUDGET = 3000 ms` for every Qt6
  app in the verification matrix (kate, dolphin, krita,
  qbittorrent, lutris, shadPS4QtLauncher). One additional descend per
  item is acceptable; quadratic blowup is not.
- **NFR-004 Backwards-compatible JSON schema.** `active.json`
  shape MUST NOT change. Any new field (e.g. `focused_output` for
  FR-006) is OPTIONAL and the QML side MUST tolerate its absence.

## Out of scope

- Hover-to-open top-level menus (gnome-style auto-cascade). Click-to-
  open stays the v1 contract; revisit in v1.1 once the geometry +
  focus tracking is solid.
- Alt-letter mnemonics / keyboard navigation. Constitution principle
  excludes from v1.
- Hyprland / KWin / Sway focus tracking. Constitution principle I.
- Replacing AT-SPI substrate with another protocol. ADR-0024 stands.
- Rewriting `_failedState` envelope into a proper state machine. The
  current latch + self-clear is sufficient; a state-machine refactor
  is its own spec.
- Changing the v1.0.x line numbering. This ships as v1.0.1.

## Constraints / dependencies

- Self-hosted runner availability (`vm103.home302server` and
  `desktop`) gates CI turnaround; both must be online for the PR
  series to merge same-day.
- `niri-ipc` crate version pin (currently inherited from `bridge/
  Cargo.toml`) — no bump in this hotfix.
- `Quickshell` v0.3.0 layer-shell semantics — assumed unchanged in
  the patch window.
- `step-security/harden-runner@<sha>` egress allowlist — adding a new
  fixture-pull URL would require a workflow review; FR-001 fixture
  MUST be committed under `bridge/tests/fixtures/`.
- DCO sign-off + Conventional Commits + lefthook all unchanged.
- Repository Rulesets on `main` block direct pushes — fix lands via
  PR per usual workflow.

## Success criteria

- **SC-001** With `shadPS4QtLauncher` focused, every top-level menu
  click opens a dropdown sized to its content (≥ widest label + 2 ×
  marginM), with a visible border, anchored under the clicked button.
  Manual smoke + screenshot diff against the broken state.
- **SC-002** With `shadPS4QtLauncher` focused, every depth-≥-2
  cascade (`View > Game List Mode > List`) opens within one click,
  rendering the real leaf labels. Manual smoke + screenshot.
- **SC-003** With a popup open, clicking another top-level button on
  the bar transitions the popup to the new anchor without losing the
  click (no double-click required). Manual smoke against
  `shadPS4QtLauncher`'s File / View / Settings / Help row.
- **SC-004** Bridge unit test fixture
  `bridge/tests/fixtures/qt_nested_wrapper.json` round-trips through
  the recursive flatten and produces the expected non-wrapped tree.
  Asserted in `cargo test`.
- **SC-005** Plugin qmltest harness
  `plugin/tests/qmltest/submenu_cascade.qml` opens a depth-3 cascade
  and asserts every `SubmenuPopup` is `visible == true`. Run in CI
  via existing `qmltest` invocation.
- **SC-006** Verification matrix passes against five Qt6 apps from the
  constraint list (kate, dolphin, krita, qbittorrent, lutris) — at
  least three MUST render correct submenus at depth ≥ 2 and accept
  click activations end-to-end. Pedro signs off via PR review.
- **SC-007** No regression in the existing focus-change flicker test
  (ADR-0019 reserveSlot stable-slot). `qmllint` + the existing
  qmltest suite stays green.
- **SC-008** Released as `v1.0.1` (or `v1.0.1-rc.1` then `v1.0.1`)
  with `actions/attest-build-provenance` + CycloneDX 1.6 SBOM
  intact, per ADR-0026. `gh attestation verify` on the bridge binary
  exits 0.

When all SCs pass, the spec is "shipped" and the slug moves from
`specs/009-popup-hotfix/` into the `v1.0.1` Git tag's release notes.
