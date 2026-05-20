# Tasks — spec 015 ship-ready completion

**Spec:** `specs/015-ship-ready-completion/spec.md`
**Plan:** `specs/015-ship-ready-completion/plan.md`

Each task is dependency-ordered. `Owner` is the agent kind that
should execute. Each task closes one or more FRs.

## Phase 1 — Bridge accelerator scaffold

### T1.1 — Add `keybinding` to MenuItem + walker

- **Owner:** rust dev (bridge)
- **Files:** `bridge/src/atspi.rs`
- **Changes:** Extend `MenuItem` struct with
  `keybinding: Option<String>`. In the per-leaf walker,
  call `accessible.action_iface()?.get_key_binding(0)` and
  store the result. Add `#[serde(skip_serializing_if =
  "Option::is_none")]` so empty stays out of the wire.
- **Tests:** New unit test `keybinding_round_trip` covering
  serialize / deserialize of a `Ctrl+T` value.
- **FR closed:** Bridge half of FR-003.
- **Blocks:** T1.2, T2.4.

### T1.2 — Bridge `atspi-click` grows `--focus-settle-ms`, `--mode`, `--keybinding`

- **Owner:** rust dev (bridge)
- **Files:** `bridge/src/main.rs`
- **Changes:**
  - `clap` enum `ClickMode { Atspi, Accelerator, Auto }`,
    default `Auto`.
  - `--focus-settle-ms u64` default 150.
  - `--keybinding String` optional.
  - `run_atspi_click(service, path, winid, settle_ms, mode,
    keybinding)`:
    - `Auto`: route to `Accelerator` if `keybinding` is
      Some+non-empty; else `Atspi`.
    - `Accelerator`: invoke niri keyboard-input dispatch
      (research niri 26.4 IPC; fall through to AT-SPI on
      missing surface).
    - `Atspi`: existing path with `settle_ms` replacing the
      30 ms constant.
  - INFO log line per spec FR-002.
- **Tests:** Bridge subprocess smoke against a synthetic
  AT-SPI fixture (cargo + bridge integration test).
- **FR closed:** FR-001, FR-002, bridge half of FR-003.
- **Blocks:** T2.5.

### T1.3 — active.json schema v2

- **Owner:** rust dev (bridge)
- **Files:** `bridge/src/proxy.rs`
- **Changes:** Bump `ACTIVE_JSON_SCHEMA_VERSION` to 2; write
  `keybinding` into each leaf object. Downstream readers
  tolerate unknown keys; legacy v1 readers ignore.
- **Tests:** Snapshot-write test confirms the key is present
  when MenuItem.keybinding is Some.
- **FR closed:** FR-003 wire format.

## Phase 2 — Plugin work

### T2.1 — Plugin reads `keybinding`

- **Owner:** qml dev (plugin)
- **Files:** `plugin/BarWidget.qml`, `plugin/MenuRow.qml`
- **Changes:** `applySnapshot` already propagates the menu
  tree verbatim; `keybinding` is preserved automatically.
  No code change in applySnapshot. Update MenuRow to render
  a right-aligned `keybinding` chip on rows that carry one
  (subtle, Color.mOnSurfaceVariant, Style.fontSizeXS).
- **Tests:** qmllint clean; visual smoke confirms the chip.
- **FR closed:** none directly; sets up T2.5.

### T2.2 — Self-heal cascade for submenus

- **Owner:** qml dev (plugin)
- **Files:** `plugin/SubmenuPopup.qml`
- **Changes:** Mirror BarWidget's empty-child detection.
  Add `_cascadePendingRetry`, RefreshActive Process, retry
  Timer. Bounded to one retry per cascade-open.
- **Tests:** qmllint clean; live smoke against a Firefox
  submenu that lazily realises.
- **FR closed:** FR-006.

### T2.3 — Self-heal telemetry counter

- **Owner:** qml dev (plugin)
- **Files:** `plugin/BarWidget.qml`,
  `plugin/AppmenuPopupWindow.qml`
- **Changes:** Add `_selfHealCount: int`. Bump on retry
  success / failure. Emit one summary line on popup close:
  `[appmenu] popup-close label=<L> retried=<N>`. Reset on
  next popup open.
- **Tests:** Live smoke: open File 50 times in steady state;
  grep journal for `retried=0`.
- **FR closed:** FR-005.

### T2.4 — Self-heal hard ceiling formalised

- **Owner:** qml dev (plugin)
- **Files:** `plugin/BarWidget.qml`
- **Changes:** Current single-shot retry semantics are
  load-bearing (already in v1.0.21). Add a guard
  `if (_pendingRetryButton !== null) return;` at the top
  of the retry-fire path so concurrent click storms can't
  pile up retries.
- **Tests:** qmllint clean; manual click-storm smoke.
- **FR closed:** FR-005 ceiling.

### T2.5 — Plugin passes `keybinding` to atspi-click

- **Owner:** qml dev (plugin)
- **Files:** `plugin/BarWidget.qml`
- **Changes:** Inside `fireClick(item)`, when
  `item.keybinding` non-empty, append `--mode accelerator
  --keybinding <value>` to the command. Otherwise pass
  `--mode auto`.
- **Tests:** Live smoke: click `File → New Tab` (Ctrl+T).
  Bridge log shows `mode=accelerator key=Ctrl+T`.
- **FR closed:** Plugin half of FR-003.
- **Depends on:** T1.1, T1.2.

## Phase 3 — Visual audit + token discipline

### T3.1 — Author visual-audit.md

- **Owner:** speckit / qml dev
- **Files:** `specs/015-ship-ready-completion/visual-audit.md`
- **Changes:** One row per VP-NNN item in
  `checklists/visual-parity.md`. Three columns: ID,
  description, current status (PASS/FAIL with line-ref).
  Initial commit reflects the v1.0.21 state.
- **Tests:** `grep -c FAIL visual-audit.md == 0`.
- **FR closed:** FR-004.

### T3.2 — Token-discipline grep

- **Owner:** ci-engineer
- **Files:** `lefthook.yml`, possibly
  `scripts/verify-tokens.sh`.
- **Changes:** New pre-commit command — scans `plugin/*.qml`
  for raw hex / pixelSize-literal / radius-literal /
  border.width-literal / duration-literal outside header
  comments. Non-zero hit → fail commit.
- **Tests:** Synthetic regression: hand-edit a literal,
  attempt commit, see refusal.
- **FR closed:** FR-010.

## Phase 4 — Gate scripts

### T4.1 — `gates/visual.sh`

- **Owner:** ci-engineer / bash
- **Files:** `specs/015-ship-ready-completion/gates/visual.sh`
- **Changes:** Bash script executing every VP-NNN check from
  `checklists/visual-parity.md`. Emits PASS/FAIL per row.
  Exits non-zero on any FAIL. Optional `--self-test` flag
  artificially regresses a value and confirms the gate
  catches it.
- **Tests:** Self-test mode exits non-zero.
- **FR closed:** part of FR-007.

### T4.2 — `gates/routing.sh`

- **Owner:** bash / qml-aware
- **Files:** `specs/015-ship-ready-completion/gates/routing.sh`
- **Changes:** Implements the 10-trial multi-Firefox-instance
  protocol from `checklists/routing-smoke.md`. Requires
  Firefox running with ≥ 3 windows; otherwise SKIP with a
  remediation pointer.
- **Tests:** Live run against the desktop host's 3 Firefox
  windows.
- **FR closed:** part of FR-007.

### T4.3 — `gates/self-heal.sh`

- **Owner:** bash
- **Files:** `specs/015-ship-ready-completion/gates/self-heal.sh`
- **Changes:** Spawns clicks via `xdotool` or equivalent
  against the bar's File button; tails journal; asserts
  retry-count == 0 in steady state. Synthetic bridge restart
  for SH-040 negative case.
- **Tests:** Live run.
- **FR closed:** part of FR-007.

### T4.4 — `gates/deploy.sh`

- **Owner:** bash
- **Files:** `specs/015-ship-ready-completion/gates/deploy.sh`
- **Changes:** Re-invokes `scripts/release.sh` with the
  same VERSION twice; asserts stage idempotence per
  `checklists/deploy-idempotence.md`. Pre-cleans `.backup`
  files per DI-050.
- **Tests:** Live run against v1.0.21 (or whatever's
  currently deployed).
- **FR closed:** part of FR-007.

## Phase 5 — Release-skill integration

### T5.1 — `scripts/verify-release.sh` driver

- **Owner:** ci-engineer / bash
- **Files:** `scripts/verify-release.sh`
- **Changes:** Sequential gate runner. Emits per-gate
  PASS/FAIL line. Writes JSON summary to
  `/tmp/noctalia-appmenu-release-gate-<TAG>.json`. Exit
  non-zero on any FAIL.
- **Tests:** All four gates wired; runs in <30 s on the
  desktop host.
- **FR closed:** FR-007 driver.

### T5.2 — Wire `verify-checklist` stage into `scripts/release.sh`

- **Owner:** ci-engineer
- **Files:** `scripts/release.sh`
- **Changes:** New `stage verify-checklist verify_checklist`
  call between `plugin-tag` and `plugin-release`. Stage
  function invokes `scripts/verify-release.sh`. Non-zero
  aborts.
- **Tests:** Synthetic regression (revert popup radius);
  run skill; observe abort.
- **FR closed:** FR-007 integration.

### T5.3 — `nixos-deploy` `.backup` pre-clear

- **Owner:** bash
- **Files:** `scripts/release.sh`
- **Changes:** Extend the existing `nixos_deploy` function
  to sweep `.backup` under `~/.claude/plugins/`,
  `~/.config/git/hooks/`, `~/.config/noctalia/` before
  invoking nixos-rebuild. Logs each removal.
- **Tests:** Plant a `.backup` file pre-run; confirm removal.
- **FR closed:** DI-050.

## Phase 6 — Governance

### T6.1 — CLAUDE.md trigger I

- **Owner:** docs / governance
- **Files:** `CLAUDE.md` (Drift detection table).
- **Changes:** Add row I:
  *"User-reported failure mode persists across ≥ 2 deploys
  against the same symptom"* with mechanical detection +
  required action (redesign spec).
- **Tests:** Markdown rendering check; lints clean.
- **FR closed:** FR-008 doctrine half.

### T6.2 — `scripts/verify-no-third-patch.sh`

- **Owner:** ci-engineer
- **Files:** `scripts/verify-no-third-patch.sh`,
  `lefthook.yml`.
- **Changes:** New pre-push hook. Scans last 5 merged PRs
  for symptom-phrase overlap with current branch title.
  ≥ 2 → refuse push with redesign-spec template path.
- **Tests:** Synthetic dry-run; recorded in `case-study.md`.
- **FR closed:** FR-008 mechanical half.

## Dependency graph (summary)

```
T1.1 → T1.2 → T1.3
              ↓
T2.5 ←——— T2.5
T2.2, T2.3, T2.4 independent
T3.1 independent
T3.2 independent

T4.* depend on T1.* + T2.* landing
T5.1 depends on T4.*
T5.2 depends on T5.1
T5.3 independent
T6.1, T6.2 independent
```

## Done definition

All tasks completed AND all four gates PASS via
`scripts/verify-release.sh` AND Pedro confirms SC-002
visually AND the v1.0.22+ release ships via the canonical
skill with no manual intervention.
