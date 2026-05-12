# Implementation plan: project completion roadmap (v0.3.0 → v1.0.0)

**Spec:** `specs/004-project-completion/spec.md`
**Constitution version:** 1.0.0
**Generated:** 2026-05-12

## Approach

Spec 004 is the *umbrella* spec — 29 functional requirements span six independent code lanes (bridge focus tracker, bridge AT-SPI walker, QML plugin, Nix surface, CI/release, quality gate + docs). Implementing them serially under one PR would violate the constitution's "≤ 25 tasks per spec" cap (Development Workflow §3) and would force one reviewer to context-switch across Rust, QML, Nix, YAML, and Markdown in a single review.

The implementation strategy ladders down: spec 004 stays the umbrella; each lane spawns its own child sub-spec (`005-bridge-completion`, `006-plugin-completion`, `007-nix-completion`, `008-ci-quality-docs`) with its own `plan.md`, `tasks.md`, and PR. Each child sub-spec branches off `origin/main`, not off `004-project-completion`; the umbrella branch stays mergeable as soon as the spec docs are reviewed, and child branches land independently in a dependency order documented under §Rollout.

Parallelization is achieved by spawning four `claude --print` worker sessions in parallel (the proven pattern documented in `~/Documents/Code/CLAUDE.md` §Multi-agent orchestration — verified 23/04/2026, spec 016 portfolio refresh, 5 commits in 354 s wall). Each worker owns one lane, gets a self-contained brief, an allowlisted Bash surface, a per-lane budget cap, and a stream-json log. The parent (this Claude Code session) waits for each worker's session_id and final result, reviews the resulting feature branch, and creates the PR after a manual diff inspection. Workers are explicitly forbidden from opening their own PRs or pushing to `main`.

The four-lane split is chosen to minimise file-collision risk. Lane A (bridge) is the only lane touching `bridge/src/*.rs`; Lane B (plugin) is the only lane touching `plugin/*.qml`; Lane C (Nix) is the only lane touching `nix/*.nix` + `flake.nix`; Lane D (CI/quality/docs) is the only lane touching `.github/workflows/*` + `sonar-project.properties` + `README.md`. Cross-lane dependencies are minimal and documented under §Risks — most importantly, Lane D's FR-022 AT-SPI integration test consumes Lane A's deliverable, so Lane D is the last to merge.

Reference ADRs that govern this plan: ADR-0011 (HM module scope — v1.0.0 stays HM-only), ADR-0013 (runner-agnostic CI), ADR-0015 (v0.1 fallback-only shipping — graceful-degradation lineage), ADR-0017 (plugin manifest schema), ADR-0018 (bar widget API contract), ADR-0019 (always-visible bar widget), ADR-0024 (AT-SPI substrate — supersedes ADR-0022/0023). Constitution principles I, II, V, VI, VII drive the lane boundaries; principle III (worktree-first) shapes the parallelization mechanic.

## Constitution Check

| Principle | Status | Notes |
|---|---|---|
| I — niri-only v1 | PASS | FR-003 opens an abstraction door but does not implement Hyprland/Sway. No compositor-agnostic code lands in v1.0.0. |
| II — Sidecar by default | PASS | No bus-name acquisition or D-Bus server work moves into QML. Lane A keeps it in Rust; Lane B consumes the existing fixed proxy. |
| III — Worktree-first git | PASS | Each of the four worker shells creates its own `../noctalia-appmenu-NN-slug/` worktree off `origin/main`; the parent session reviews from its own worktree. |
| IV — Conventional Commits + DCO | PASS | Briefs explicitly mandate `git commit -s` + conventional-commit subjects; lefthook enforces both. |
| V — Speckit-driven | PASS | Each lane runs its own `speckit.plan → tasks → implement` cycle. No code lands outside a written sub-spec. |
| VI — Release-engineering compliance | PASS | FR-021 fixes the CycloneDX 1.6/1.7 mismatch that currently violates the standard; no new deviations introduced. |
| VII — Graceful degradation | PASS | FR-004 (GTK4 empty children → synthetic fallback), FR-005 (AT-SPI bus restart recovery), FR-007 (stale-path click typed-error) all add new degradation paths instead of new error-out paths. |

All seven gates green. No FAIL, no waiver required.

## Architecture sketch

```
                          spec 004 (umbrella)
                                  |
            +---------------------+---------------------+---------+
            |                     |                     |         |
       LANE A (Rust)          LANE B (QML)         LANE C (Nix)   |
   005-bridge-completion  006-plugin-completion  007-nix-completion
            |                     |                     |
   FR-001..FR-009         FR-010..FR-013         FR-014..FR-020
   bridge/src/*.rs        plugin/*.qml           nix/module.nix
                                                 flake.nix
                                                                  |
                          LANE D (CI + quality + docs)  ----------+
                          008-ci-quality-docs
                          FR-021..FR-029
                          .github/workflows/*
                          sonar-project.properties
                          README.md

   merge order:           A → B (no hard dep) → C → D last
                          (D's FR-022 AT-SPI integration test
                          consumes A's deliverable)
```

Per-lane child-spec layout (each independently follows the `speckit.specify → plan → tasks → implement` recipe):

- `specs/005-bridge-completion/` — `niri.rs`, `focus.rs` (new), `atspi.rs`, deletion of `dbusmenu.rs` + `registrar.rs`.
- `specs/006-plugin-completion/` — `BarWidget.qml`, `AppmenuPopupWindow.qml`, `SubmenuPopup.qml` (new).
- `specs/007-nix-completion/` — `nix/module.nix`, `flake.nix`.
- `specs/008-ci-quality-docs/` — workflows, `sonar-project.properties`, `README.md`, optionally `docs/adr/ADR-0025.md` (cognitive-complexity deviation, only if FR-027 refactor deferred).

## Affected files

### Spec 004 itself (this PR)

- `specs/004-project-completion/spec.md` (created, 309 lines)
- `specs/004-project-completion/research.md` (created, 268 lines)
- `specs/004-project-completion/checklists/requirements.md` (created, 42 lines)
- `specs/004-project-completion/plan.md` (this file, created)
- `specs/004-project-completion/data-model.md` (created in Phase 1)
- `specs/004-project-completion/contracts/*` (created in Phase 1)
- `specs/004-project-completion/quickstart.md` (created in Phase 1)

### Spec 005 (Lane A — bridge)

- `bridge/src/focus.rs` (new — `FocusSink` trait, `FocusEvent`, `FocusOp`)
- `bridge/src/niri.rs` (modified — backoff reset, integration tests for ack-path)
- `bridge/src/atspi.rs` (modified — GTK4 empty-children fallback, `IsEnabled` monitoring, persistent connection, click re-fetch, app-id round-trip tests)
- `bridge/src/lib.rs` (modified — `mod dbusmenu` / `mod registrar` removed)
- `bridge/src/dbusmenu.rs` (deleted)
- `bridge/src/registrar.rs` (deleted)
- `bridge/tests/atspi_integration.rs` (new — fake AT-SPI registry stub harness)
- `bridge/tests/niri_reconnect.rs` (new — reconnect + backoff reset coverage)

### Spec 006 (Lane B — plugin)

- `plugin/SubmenuPopup.qml` (new — nested popup, sibling layer-shell surface, ADR-0008)
- `plugin/AppmenuPopupWindow.qml` (modified — `hasChildren` click → open `SubmenuPopup`, `toggle_state` rendering, `icon_name` rendering, multi-screen guard)
- `plugin/BarWidget.qml` (modified — pass focused-output screen to popup, `toggle_state` rendering in top-level strip if applicable)
- `plugin/qmldir` (new or modified — register `SubmenuPopup`)

### Spec 007 (Lane C — Nix)

- `nix/module.nix` (modified — `QT_ACCESSIBILITY=1`, AT-SPI assertion, deprecate `registrar` option + `vala-panel-appmenu` deps, remove stale `QT_QPA_PLATFORMTHEME` / `GTK_MODULES`, plugin discovery wiring)
- `flake.nix` (modified — version from `Cargo.toml`, `SOURCE_DATE_EPOCH` from outside the sandbox)
- `nix/version.nix` (new — shared version source-of-truth, optional)
- `nix/options.md` (new — generated option documentation, optional)

### Spec 008 (Lane D — CI / quality / docs)

- `.github/workflows/release.yml` (modified — CycloneDX 1.6 → 1.7)
- `.github/workflows/ci.yml` (modified — qmllint SARIF emit + upload; AT-SPI integration test job)
- `.github/workflows/actionlint.yml` (modified — runner-agnostic label)
- `.github/rulesets/main.json` (modified — required-checks list per FR-025)
- `sonar-project.properties` (modified — coverage floor, complexity, duplication thresholds; new-code gate in UI)
- `README.md` (modified — "Verify the install" 10-min recipe; documented caveats per FR-029)
- `docs/adr/ADR-0025-cognitive-complexity-waiver.md` (optional, only if FR-027 refactor is deferred)

## Risks

- **R1** Lane A's `Cargo.lock` may collide with Dependabot #67 (zbus-stack group) if both rebase against `main` independently. *Mitigation*: parent merges #67 (or closes it) before dispatching Lane A; Lane A brief acknowledges the rebase order.
- **R2** Lane B's `SubmenuPopup.qml` cannot be fully smoke-tested without Lane A's AT-SPI integration test (FR-022) — the submenu needs real menu data to render. *Mitigation*: Lane B's child spec ships a fixture-driven QML test (`tests/qmltest/submenu_popup.qml`) using a hand-crafted JSON tree; full-stack verification happens post-merge under Lane D's integration job.
- **R3** Lane C's deprecation of the `registrar` option may break existing users who set `registrar = "vala-panel"` explicitly. *Mitigation*: FR-016 keeps the option recognised for one cycle (v1.0.0) with `lib.warn`; removal happens in v1.1.
- **R4** Lane D's CycloneDX 1.7 fix is in the release workflow only; users who already verified v0.3.0 will see the wrong SBOM format. *Mitigation*: v1.0.0 release notes explicitly list this as a fixed defect; no re-tagging of v0.3.0 (constitution invariant: never re-tag).
- **R5** Parallel worker shells may inflate cumulative cost beyond budget. *Mitigation*: per-lane `--max-budget-usd` caps ($10/$6/$5/$7); total ≤ $28; the parent kills any worker that exceeds 80 turns without producing a commit.
- **R6** The four-worker fanout exceeds the `~/Documents/Code/CLAUDE.md` anti-pattern threshold ("Spawn 5+ children for one feature — coordination overhead exceeds savings"). *Mitigation*: stays at four, not five+; each worker is self-contained and reports back via stream-json, so coordination is read-only on the parent side.
- **R7** Per-lane child specs may individually exceed the constitution's 25-task cap. *Mitigation*: each child spec's `tasks.md` is reviewed at planning time; if any lane exceeds, it splits further (e.g. Lane A could split into `005-bridge-focus` + `006-bridge-atspi`).

## Rollout

1. **Spec 004 PR** (this work): merge the umbrella spec + plan + research + data-model + contracts + quickstart. No code changes. Reviewer ensures the constitution check stays green and the lane boundaries are sound.
2. **Dependabot triage** (parent task, pre-lane-A): merge or close #65, #66, #69, #71 (safe patches); review #67 (zbus-stack — touches Lane A); review #70, #72; defer #68, #64 (major bumps) to post-v1.
3. **Lane A — spec 005** (parent dispatches worker; worker creates `73-bridge-completion` worktree): implements FR-001..FR-009, opens PR; parent reviews + merges. Tags lane complete when bridge integration test green in CI.
4. **Lanes B + C in parallel** (parent dispatches two workers simultaneously after Lane A merge): spec 006 + spec 007 land independently. Reviewer focuses on QML and Nix.
5. **Lane D — spec 008** (parent dispatches after B+C merged; worker creates `76-ci-quality-docs` worktree): implements FR-021..FR-029, including the AT-SPI integration test that exercises Lane A's deliverable end-to-end.
6. **v1.0.0 tag**: when all eight success criteria (SC-001..SC-008) pass on `main`, `git-cliff` generates the changelog and the release workflow tags `v1.0.0`. No re-tagging if a defect is found post-tag — `v1.0.1` follows.

## Open questions

1. Should the v1.0.0 quality gate (FR-026) be enforced as a *required* status check on the Repository Ruleset, or remain advisory until the project has soaked under Pedro's daily use for two weeks post-tag? *Default*: required at tag time; demote to advisory only if soak shows false-positive churn.
2. Does noctalia-shell's plugin loader scan directories or require an explicit `~/.config/noctalia/plugins.json` entry? *Status*: FR-020 leaves this open. Lane C's brief includes "verify upstream behaviour first; if directory-scanning works, no `plugins.json` write is needed".
