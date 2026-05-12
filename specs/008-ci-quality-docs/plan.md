# Implementation plan: CI + quality gate + documentation (Lane D)

**Spec:** `specs/008-ci-quality-docs/spec.md`
**Parent:** `specs/004-project-completion/plan.md` §Approach (Lane D)
**Constitution version:** 1.0.0
**Generated:** 2026-05-12

## Approach

Lane D is the smallest and most localised of the four child lanes. Every change lands under one of five files (`release.yml`, `ci.yml`, `actionlint.yml`, `sonar-project.properties`, `README.md`) plus two new files (`.github/rulesets/main.json`, `docs/adr/ADR-0025-cognitive-complexity-waiver.md`). Total blast radius is < 200 lines of diff.

The lane orders the work to minimise CI churn:

1. **Workflow edits first** (FR-021, FR-023, FR-024, FR-022). Each can be validated locally by `actionlint` + `zizmor` before push.
2. **Ruleset JSON next** (FR-025). Adds a checked-in record of the required-checks set; live GitHub config is applied out-of-band.
3. **Sonar properties + ADR-0025** (FR-026, FR-027). No workflow churn; pure config + docs.
4. **README last** (FR-028, FR-029). The biggest doc change is the rewrite of the "Verify the install" section to match the AT-SPI substrate; consolidating it at the end avoids a partial-doc state mid-lane.

FR-022 is the only cross-lane dependency in this set. Lane A owns `bridge/tests/atspi_integration.rs`; Lane D wires the CI job. The implementation pattern is a file-existence gate: if the test file is missing, the job emits a `notice` and exits 0; if it is present, it shells out to `cargo test --test atspi_integration`. This decouples Lane D's merge from Lane A's merge and keeps the required-check green throughout.

The lane explicitly does NOT refactor `bridge/src/atspi.rs` even though FR-027 cites the cognitive-complexity hot paths. Lane A owns `atspi.rs` and may take either path (refactor under spec 005, or extend the waiver). Lane D ships only the waiver ADR-0025 so the quality gate stays green if Lane A's refactor slips past `v1.0.0`.

## Constitution Check

| Principle | Status | Notes |
|---|---|---|
| I — niri-only v1 | PASS | No compositor abstraction touched. |
| II — Sidecar by default | PASS | No QML or D-Bus changes in this lane. |
| III — Worktree-first git | PASS | Lane works in `../noctalia-appmenu-77-ci-quality-docs/`; main worktree untouched. |
| IV — Conventional Commits + DCO | PASS | All commits `git commit -s` with `ci(scope):` / `docs(scope):` / `chore(scope):` subjects. |
| V — Speckit-driven | PASS | Lane runs its own `spec → plan → tasks` cycle under `specs/008-ci-quality-docs/`. |
| VI — Release-engineering compliance | PASS | FR-021 fixes the CycloneDX 1.6 → 1.7 drift; no new deviations introduced. Action pins remain SHA + comment. |
| VII — Graceful degradation | PASS | FR-022's file-existence gate is a graceful degradation: missing fixture → no-op success, not red CI. |

All seven gates green.

## Architecture sketch

```
.github/workflows/release.yml          — FR-021 (CycloneDX 1.7)
.github/workflows/ci.yml               — FR-022 (atspi-integration job)
                                       — FR-024 (qmllint SARIF + upload)
.github/workflows/actionlint.yml       — FR-023 (drop desktop label)
.github/rulesets/main.json             — FR-025 (required-checks JSON export)
sonar-project.properties               — FR-026 (v1 thresholds)
docs/adr/ADR-0025-…                    — FR-027 (waiver)
README.md                              — FR-028 (Verify the install)
                                       — FR-029 (caveats)
```

## Affected files

| File | Change | FR |
|---|---|---|
| `.github/workflows/release.yml` | line 77: `cyclonedx-json@1.6` → `@1.7`; add an assertion step that parses the emitted SBOM | FR-021 |
| `.github/workflows/ci.yml` | new `atspi-integration` job (file-existence-gated); rewrite `plugin-lint` job to glob `plugin/*.qml`, emit SARIF, upload via `codeql-action/upload-sarif` | FR-022, FR-024 |
| `.github/workflows/actionlint.yml` | line 33: drop `desktop` label; rewrite the inline comment to point at the upstream runner-hook fix | FR-023 |
| `.github/rulesets/main.json` (new) | checked-in JSON export of the live Ruleset on `main`, required-checks per `contracts/ci-required-checks.md` | FR-025 |
| `sonar-project.properties` | bump `sonar.coverage.minimum` 60 → 65; document new-code-80 % gate in a comment; document duplication < 3 % + blockers = 0 thresholds; keep cognitive-complexity ≤ 15 (already correct) | FR-026 |
| `docs/adr/ADR-0025-cognitive-complexity-waiver.md` (new) | Accepted waiver for `find_app_for_pid` + `fetch_menu_tree`; expiry tied to Lane A's refactor OR v1.0.1 | FR-027 |
| `README.md` | replace stale v0.1 substrate description (DBusMenu/Registrar) with AT-SPI substrate (ADR-0024); add `## Verify the install` recipe; add `## Caveats` section | FR-028, FR-029 |

## Risks

- **R-D1** Lane A may decide to refactor instead of waiver. *Mitigation:* the waiver ADR-0025 is harmless if the refactor lands first — the Sonar gate stops citing the functions and the ADR's "Expires when" trigger fires.
- **R-D2** FR-022's file-existence gate could let a Lane A regression land silently if the fixture file is deleted by accident. *Mitigation:* once Lane A merges, a separate PR removes the gate so absence becomes a hard failure. Tracked in the lane's `tasks.md` as a post-Lane-A follow-up.
- **R-D3** `release.yml`'s assertion step (FR-021 verification) requires `jq` on the runner. *Mitigation:* `nix develop --command bash -c '…'` provides `jq` via the devShell; the existing `Generate cargo-cyclonedx SBOM` step already uses `nix develop`, so the toolchain is proven available.
- **R-D4** Repository Ruleset bootstrap requires `enforcement: disabled` first per `~/NixOS/meta/yolo-labz-release-engineering-research.md`. *Mitigation:* the JSON export documents `enforcement: active` as the steady state; Pedro applies via `gh api` in the documented two-step bootstrap.

## Rollout

1. Worktree + branch `77-ci-quality-docs` off `origin/main`.
2. Write spec / plan / tasks / checklist under `specs/008-ci-quality-docs/`.
3. Edit workflows + Sonar + ADR + README in the order above.
4. `actionlint -color` + `zizmor --format=plain` clean.
5. DCO-signed conventional commits, one per FR cluster (release / ci / actionlint / ruleset / sonar / adr / readme).
6. Push branch.
7. Parent (the orchestrating Claude session) reviews and opens the PR.

## Open questions

- The cargo-machete workflow still hard-pins to `desktop` per its inline comment. Lane D does not edit it (out of scope for FR-023, which scopes the hard-pin removal to `actionlint.yml` specifically). Pedro decides whether a follow-up lane unrolls cargo-machete's pin too — captured as a non-blocking note in this plan, not a Lane D acceptance gate.
