# Implementation plan: v1.0.0 popup hotfix — submenu cascade + render fidelity

**Spec:** `specs/009-popup-hotfix/spec.md`
**Constitution version:** 1.0.0

## Approach

Eight functional requirements split cleanly along the bridge ↔ plugin
seam: FR-001 lives in `bridge/src/atspi.rs` (recursive Qt
wrapper-flatten); FR-002..005, FR-007, FR-008 live in
`plugin/{AppmenuPopupWindow,SubmenuPopup,BarWidget,MenuRow}.qml`;
FR-006 spans both (bridge writes new optional `focused_output`
field, QML reads it as a fallback). The fix is mechanical and
preserves every architectural decision in ADR-0008 (sibling
layer-shell popups), ADR-0009 (focus debouncing), ADR-0010
(graceful degradation), ADR-0024 (AT-SPI substrate).

Two parallel PR lanes:

- **Lane Q (QML)** ships FR-002..005, FR-007, FR-008 and the
  consume-side of FR-006. Single PR titled
  `fix(qml): popup geometry, cascade, dedup (#NN)` against
  `plugin/`. No bridge changes. Lands first because Pedro can
  smoke-test against any Qt6 app and it has no protocol surface.
- **Lane B (Bridge)** ships FR-001 and the produce-side of FR-006.
  Single PR titled `fix(bridge): recursive Qt menu flatten + emit focused_output (#NN)`
  against `bridge/`. Adds one fixture
  (`bridge/tests/fixtures/qt_nested_wrapper.json`) and one new field
  in `active.json` (optional, schema-additive — see NFR-004 + ADR-
  0024 §Migration).

Both PRs target `main`. Order is independent — Lane Q tests against
the existing v1.0.0 bridge (broken submenu data, but the QML fixes
manifest with kate / dolphin / krita whose Qt menus happen to flatten
on the first level too); Lane B tests against the existing v1.0.0
QML (bridge fix won't visually fix Pedro's screenshot until Lane Q
also lands). Once both merge, cut `v1.0.1`.

The fix path stays inside the existing supply-chain spine
(ADR-0026 CycloneDX 1.6, attest-build-provenance v4.1.0,
SOURCE_DATE_EPOCH, harden-runner audit). No workflow changes. No new
egress. No re-tagging.

## Constitution Check

| Principle | Status | Notes |
|---|---|---|
| I — niri-only v1 | PASS | Hotfix scoped to niri only; no Hyprland / KWin paths added. FR-006 fallback uses `active.json` field, which the bridge populates from niri-IPC. |
| II — Sidecar by default | PASS | Bridge owns wrapper-flatten + focused_output emission. QML owns hit-testing + geometry. No QML-side D-Bus claim. |
| III — Worktree-first git | PASS | Spec authored in `~/Documents/Code/yolo-labz/noctalia-appmenu-009-popup-hotfix`; main worktree untouched. |
| IV — Conventional Commits + DCO | PASS | Lane Q `fix(qml):`, Lane B `fix(bridge):`, both DCO-signed. lefthook + commitlint enforce on PR. |
| V — Speckit-driven | PASS | This is the spec → plan flow. Tasks ≤ 25 (Phase 2 will produce 14–18). |
| VI — Release-engineering compliance | PASS | Releases as v1.0.1 on the existing v1.0.x supply-chain spine. SC-008 keeps attest + SBOM (CDX 1.6 / SPDX 2.3) intact. No workflow changes. |
| VII — Graceful degradation | PASS | FR-006 + FR-008 strengthen graceful-degradation invariants. FR-002 (constrained popup surface) does not introduce any path that crashes the bar. |

No FAILs. No ADR amendments needed. No constitution amendments needed.

## Architecture sketch

```
+--------------------------+
|  niri-IPC focus stream   |
+-----------+--------------+
            |
            v
+--------------------------+        +-----------------------------+
| bridge: focus.rs         | -----> | bridge: atspi.rs            |
|  (debounce, FR-006 src)  |        |  (FR-001 recursive flatten) |
+-----------+--------------+        +-------------+---------------+
            |                                     |
            v                                     v
   +--------------------------+    +-----------------------------+
   | bridge: active.rs        |--->| ~/.cache/noctalia-appmenu/   |
   |  + focused_output field  |    |   active.json (schema add)   |
   +--------------------------+    +-------------+---------------+
                                                 |
                                                 v IpcHandler push +
                                                 v FileView fallback
                                  +-----------------------------+
                                  | plugin: BarWidget.qml       |
                                  |  - FR-005 dedup w/ children  |
                                  |  - FR-006 read focused_output |
                                  |  - FR-008 _failedState clear  |
                                  +------+----------+------------+
                                         |          |
                                         v          v
                       +---------------------+  +-----------------------+
                       | AppmenuPopupWindow  |  | SubmenuPopup (recur)  |
                       |  FR-002 surface size |  |  FR-002 surface size  |
                       |  FR-003 width binding |  |  FR-003 width binding |
                       |                       |  |  FR-004 Loader async  |
                       |                       |  |  FR-007 ns uniq       |
                       +---------------------+  +-----------------------+
                                          \        /
                                           v      v
                                   +--------------------+
                                   | MenuRow (delegate) |
                                   +--------------------+
```

## Affected files

### Lane B (Bridge)

- `bridge/src/atspi.rs` — recursive wrapper-flatten in
  `fetch_menu_tree` (FR-001)
- `bridge/src/active.rs` — emit optional `focused_output` field
  (FR-006 produce-side)
- `bridge/src/focus.rs` — surface focused output name from niri-IPC
  to the snapshot writer (FR-006 producer plumbing)
- `bridge/tests/fixtures/qt_nested_wrapper.json` — new fixture for
  recursive flatten round-trip (SC-004)
- `bridge/tests/atspi_flatten.rs` — new test (or extend existing)
  asserting the round-trip
- `Cargo.toml` — bump `version = "1.0.1"` (release vehicle)

### Lane Q (Plugin)

- `plugin/AppmenuPopupWindow.qml` — drop full-screen anchors; size
  to `menuBox.height`; explicit y; fix `popupCol` width binding
  (FR-002, FR-003, FR-006 consume-side)
- `plugin/SubmenuPopup.qml` — same surface fix (FR-002, FR-003);
  add depth-suffix to `WlrLayershell.namespace` (FR-007); wait for
  `Loader.status === Loader.Ready` before `open()` (FR-004)
- `plugin/BarWidget.qml` — narrow `_sameTopLevel` to honour child
  changes (FR-005); resolve `focusedScreenName` with `active.json`
  fallback (FR-006 consume-side); ensure `_failedState` self-clears
  (FR-008)
- `plugin/MenuRow.qml` — no changes expected (read-only audit
  during FR-001 verification)
- `plugin/tests/qmltest/submenu_cascade.qml` — new harness
  exercising depth-3 cascade + bar-area click pass-through (SC-005)
- `plugin/tests/qmltest/popup_geometry.qml` — new harness asserting
  width clamp lift (FR-003)

### Both lanes

- `docs/adr/` — no new ADR required (changes implement existing
  ADR contracts). If FR-002 reveals a deeper protocol concern,
  open ADR-0028 in a follow-up; not in scope here.
- `CHANGELOG.md` — auto-generated by `git-cliff`; do not hand-edit.
- `specs/009-popup-hotfix/contracts/` — three contracts (this PR
  series): `recursive-flatten.md`, `popup-surface.md`,
  `active-json-schema.md` (schema-additive update).

## Risks

- **Risk 1: Constraining popup surface breaks anchor coordinates.**
  *Why:* `mapToItem(null, 0, 0)` works today because both PanelWindows
  fill the screen, sharing a coord origin. Switching the popup window
  to `menuBox.height` changes its origin. *Mitigation:* compute
  popup `y` from the bar-button's screen-absolute position via
  `mapToGlobal(0,0)` (Qt 5.7+ portable); add `popup_geometry.qml`
  test that asserts on-screen pixel coords. Tested against Pedro's
  multi-screen setup before landing.
- **Risk 2: Recursive flatten introduces quadratic blow-up on deep
  menu trees.** *Why:* every descend adds one walk. *Mitigation:*
  flatten happens BEFORE recursion's children fetch (bottom-up
  inside the existing single recursion), so descend count is
  unchanged. NFR-003 budget assertion in `atspi_flatten.rs` test
  guards against drift.
- **Risk 3: `_sameTopLevel` widening triggers extra delegate
  rebuilds, reintroducing PR #51's flicker.** *Mitigation:* compare
  full-tree structurally cheaply (children-length + first-level
  ids/labels), only rebuild when shape actually differs. Re-verify
  the AMD-flicker repro from the BarWidget.qml header comment
  before merging Lane Q.
- **Risk 4: Loader async wait silently never fires (statusChanged
  not emitted on synchronous Loaders).** *Mitigation:* check
  `Loader.status === Loader.Ready` immediately after assigning
  `sourceComponent`; only fall through to a `Connections` listener
  when status is `Loading`. Belt-and-braces.
- **Risk 5: New `focused_output` field breaks downstream consumers.**
  *Why:* anyone consuming `active.json` directly. *Mitigation:* the
  field is OPTIONAL and additive (NFR-004). The QML side defaults to
  empty string when absent.
- **Risk 6: Self-hosted runner contention delays both PRs.**
  *Mitigation:* land Lane Q first (no bridge unit-test surface), Lane B
  after, sequential CI.

## Rollout

- Dev cycle per lane:
  - Lane Q: `qmllint plugin/` clean + `qmltest plugin/tests/qmltest/`
    green + manual smoke against shadPS4QtLauncher and one of
    {kate, dolphin, krita}.
  - Lane B: `cargo test -p noctalia-appmenu-bridge` green +
    `nix flake check` green + manual smoke against
    shadPS4QtLauncher confirms `View > Game List Mode` populates.
- Land Lane Q first, then Lane B. Each `gh pr merge --squash
  --delete-branch` after CI green and Pedro signoff.
- Bump `bridge/Cargo.toml` to `1.0.1` in Lane B (Lane Q does not
  carry a release).
- Tag `v1.0.1` after both merges. Release workflow auto-attests +
  SBOMs + uploads release assets per existing pipeline (ADR-0026).
- Update `~/NixOS/flake.lock` to point to `v1.0.1` commit; deploy
  to Pedro's desktop via `nh os switch .`; confirm SC-001..006
  manually.

## Open questions

None. Spec passed validation with zero `[NEEDS CLARIFICATION]`
markers. Four-agent investigation supplied enough ground truth to
fill every ambiguity with a defensible default.
