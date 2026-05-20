# Implementation plan — spec 015 ship-ready completion

**Spec:** `specs/015-ship-ready-completion/spec.md`
**Branch:** `114-ship-ready-completion`
**Author:** @phsb5321
**Date:** 2026-05-19

## Architecture

This spec is mostly *additive* — no architectural pivots. Three
axes:

1. **Bridge — routing reliability** (FR-001, FR-002, FR-003).
   The atspi-click subcommand grows `--focus-settle-ms`, INFO
   logging, and an accelerator-key dispatch mode. `active.json`
   schema bumps to v2 with the per-leaf `keybinding` field.
2. **Plugin — self-heal coverage** (FR-005, FR-006). Existing
   `_pendingRetryButton` machinery is hardened with a per-popup
   ceiling and replicated for submenu cascades.
3. **Tooling — release-gate verification** (FR-004, FR-007,
   FR-008, FR-010). New `scripts/verify-release.sh` driver,
   four gate scripts (visual / routing / self-heal / deploy),
   CLAUDE.md trigger I extension, pre-commit hook reinforcement.

## Module-level changes

### `bridge/src/main.rs`

- `Cmd::AtspiClick` grows `focus_settle_ms: u64` (clap arg,
  default 150). Replaces the v1.0.20 hard-coded 30 ms.
- `run_atspi_click` accepts the settle param; passes to the
  sleep step.
- New mode flag `--mode {atspi,accelerator,auto}` (default
  `auto`). In `auto`, the click handler inspects the
  serialised `keybinding` field on the menu node (passed via
  a new `--keybinding "Ctrl+T"` arg) and dispatches via niri
  keyboard-input when present.
- INFO-level structured log: `[appmenu] atspi-click
  service=:1.X path=/org/a11y/atspi/accessible/Y winid=Z
  mode=atspi|accelerator settle_ms=150
  niri_focus=ok|err do_action=ok|err`.

### `bridge/src/atspi.rs`

- `MenuItem` grows a `keybinding: Option<String>` field
  (serde rename per active.json v2 key).
- Walker calls `Action.GetKeyBinding(0)` per leaf (AT-SPI
  surface). Empty string → `None`, else canonical
  `Ctrl+T` / `Ctrl+Shift+P` form.

### `bridge/src/active.rs` + `bridge/src/proxy.rs`

- `ActiveSnapshot` already carries `focus_winid` (v1.0.20).
  No further fields needed.
- Active.json bumps `v: 2`; downstream readers tolerate
  unknown keys (already the case).

### `plugin/BarWidget.qml`

- `fireClick(item)` reads `item.keybinding`. When present
  AND non-empty, passes `--mode accelerator` and
  `--keybinding "<value>"` to atspi-click.
- Self-heal counter telemetry: `_selfHealCount` int, bumped
  on retry-success or retry-fail; logged on popup close.

### `plugin/SubmenuPopup.qml`

- Mirror the BarWidget's empty-child detection. When
  `parentMenuItem.children.length === 0 && parentMenuItem.type
  === "submenu"`, kick `RefreshActive` + retry-open with a
  250 ms timer. Bounded to one retry per cascade-open session
  (same idiom as the top-level).

### `plugin/AppmenuPopupWindow.qml`

- No changes — surface treatment already complete via v1.0.18
  + v1.0.19. Visual-audit gate verifies parity.

### `scripts/release.sh`

- New stage `verify-checklist` between `plugin-tag` and
  `plugin-release`. Invokes `scripts/verify-release.sh`.
  Non-zero exit aborts.
- `nixos-deploy` stage grows a pre-clear sweep for `.backup`
  files under `~/.claude/plugins/`, `~/.config/git/hooks/`,
  `~/.config/noctalia/`. Per DI-050.

### `scripts/verify-release.sh` (NEW)

- Bash driver running each gate in sequence. Per-gate
  PASS/FAIL line to stderr, summary JSON to
  `/tmp/noctalia-appmenu-release-gate-<TAG>.json`.
- Gates:
  - `visual-smoke` — `bash specs/015-ship-ready-completion/gates/visual.sh`
  - `routing-smoke` — `bash …/routing.sh`
  - `self-heal-absence` — `bash …/self-heal.sh`
  - `deploy-idempotence` — `bash …/deploy.sh`

### Gate scripts (NEW)

Each gate script is the executable form of the matching
checklist under `specs/015-ship-ready-completion/checklists/`.
Co-locate under `specs/015-ship-ready-completion/gates/` so
review surface stays close to the contract.

### `CLAUDE.md`

- Drift detection table grows row **I**:
  *User-reported failure mode persists across ≥ 2 deploys
  against the same symptom* → action: open redesign spec,
  halt patches. Mechanical detection via `gh issue list
  --search '<symptom>'` AND `gh pr list --state merged
  --search '<symptom>'`. Both ≥ 2 → trigger fires.

### `scripts/verify-tag-subject.sh` extension

- Add a sibling `scripts/verify-no-third-patch.sh` invoked
  by lefthook pre-push. Scans last 5 merged PR titles for
  symptom-phrase overlaps with the current branch title.
  ≥ 2 matches → refuse push with the redesign-spec template
  path printed.

## Sequencing

Build sequence — each step is mergeable on its own:

1. **`bridge`-side accelerator scaffold** (no behaviour change
   yet). Grows MenuItem.keybinding, `--mode` flag, walker
   AT-SPI GetKeyBinding call. Tests: existing AT-SPI walker
   tests + one new for keybinding extraction. Ship as v1.0.22
   if a release is cut here, but ideally batch.
2. **Plugin self-heal cascade + telemetry** (FR-005, FR-006).
   Independent of bridge step. Ships in same PR or next.
3. **Visual-audit table** (FR-004). Living document; spec
   ships its initial state with PASS rows.
4. **Gate scripts** (`gates/visual.sh`, `routing.sh`,
   `self-heal.sh`, `deploy.sh`). Each implements its
   checklist. Shippable per-gate.
5. **`verify-release.sh` driver + release-skill integration**
   (FR-007). Wires gates into the skill.
6. **CLAUDE.md trigger I + pre-push extension** (FR-008).
7. **Token-discipline grep enforcement** (FR-010). Either as
   a pre-commit hook or as part of `gates/visual.sh`.

## Test strategy

- Bridge changes: `cargo test --offline --lib` against the
  walker. New unit test for `keybinding` extraction.
- Plugin changes: `qmllint` clean. Live smoke via
  `journalctl --user -u noctalia-shell | grep '\[appmenu\]'`.
- Gates: each gate script's negative-case validates itself.
  E.g. `visual.sh` should FAIL when run against a deliberately
  regressed `AppmenuPopupWindow.qml`; the spec demonstrates
  this in `gates/visual.sh`'s `--self-test` mode.
- End-to-end: `scripts/release.sh 1.0.22` against this branch
  exercises all gates before tagging.

## Risk surface

- **Bridge accelerator dispatch** depends on niri-IPC exposing
  keyboard-input actions. niri 26.4 has
  `Action::SendKeySym` / equivalent — confirm before relying.
  If absent, fall back to AT-SPI with the longer settle (still
  superior to v1.0.20).
- **Visual audit table** is human-readable but the gate is
  automated via grep. Token discipline regressions are the
  most likely false negative; the grep enforcement (FR-010)
  is the safety net.
- **Trigger I detection** is fuzzy (noun-phrase overlap). The
  initial implementation is permissive — false positives
  are noisy but harmless (operator confirms or overrides).

## Out of plan

- Cross-PID multi-window apps (Chromium with separate profile
  PIDs). Single-PID + multi-window (Firefox) close first.
- Synthetic-menu items (window-management via niri-IPC).
  Already routed correctly; spec covers AT-SPI items only.
