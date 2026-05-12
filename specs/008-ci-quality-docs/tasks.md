# Tasks: CI + quality gate + documentation (Lane D)

**Spec:** `specs/008-ci-quality-docs/spec.md`
**Plan:** `specs/008-ci-quality-docs/plan.md`
**Constitution version:** 1.0.0
**Generated:** 2026-05-12

Constitution Development Workflow §3 caps a single spec at ≤ 25 tasks. Lane D is well under that.

| # | Task | Owner | FR | Status |
|---|---|---|---|---|
| T-D01 | Worktree `../noctalia-appmenu-77-ci-quality-docs/` off `origin/main`, branch `77-ci-quality-docs` | self | — | DONE |
| T-D02 | Write `specs/008-ci-quality-docs/spec.md` | self | — | DONE |
| T-D03 | Write `specs/008-ci-quality-docs/plan.md` | self | — | DONE |
| T-D04 | Write `specs/008-ci-quality-docs/tasks.md` (this file) | self | — | DONE |
| T-D05 | Write `specs/008-ci-quality-docs/checklists/requirements.md` | self | — | TODO |
| T-D06 | Edit `.github/workflows/release.yml`: `cyclonedx-json@1.6` → `@1.7`; add SBOM-format assertion step | self | FR-021 | TODO |
| T-D07 | Edit `.github/workflows/actionlint.yml`: drop `desktop` label from `runs-on:`; rewrite inline comment | self | FR-023 | TODO |
| T-D08 | Edit `.github/workflows/ci.yml`: rewrite `plugin-lint` to glob `plugin/*.qml`, emit SARIF, upload via `codeql-action/upload-sarif` | self | FR-024 | TODO |
| T-D09 | Edit `.github/workflows/ci.yml`: add `atspi-integration` job, file-existence-gated against `bridge/tests/atspi_integration.rs` | self | FR-022 | TODO |
| T-D10 | Create `.github/rulesets/main.json` with required-checks set per `contracts/ci-required-checks.md` | self | FR-025 | TODO |
| T-D11 | Edit `sonar-project.properties`: line ≥ 65 %, document new-code 80 % UI gate, document duplication / blockers thresholds; keep complexity ≤ 15 | self | FR-026 | TODO |
| T-D12 | Create `docs/adr/ADR-0025-cognitive-complexity-waiver.md` covering `find_app_for_pid` + `fetch_menu_tree`; expires Lane A refactor OR v1.0.1 | self | FR-027 | TODO |
| T-D13 | Rewrite `README.md`: replace v0.1 substrate description (DBusMenu/Registrar) with AT-SPI substrate (ADR-0024) | self | FR-028 | TODO |
| T-D14 | Add `## Verify the install` section to `README.md` inheriting from `quickstart.md` | self | FR-028 | TODO |
| T-D15 | Add `## Caveats` section to `README.md` documenting Firefox / Electron / multi-monitor / Alt-key / GTK4 quirks | self | FR-029 | TODO |
| T-D16 | `actionlint -color` clean on all edited workflows | self | NFR-D1 | TODO |
| T-D17 | `zizmor --format=plain .github/workflows/` clean | self | NFR-D1 | TODO |
| T-D18 | DCO-signed conventional commits, one per FR cluster | self | — | TODO |
| T-D19 | Push `77-ci-quality-docs` to `origin` (no PR — parent opens it) | self | — | TODO |
| T-D20 | Post-Lane-A follow-up (separate commit, lands after Lane A merges): remove file-existence gate from `atspi-integration` job so absence = red CI | self | R-D2 | DEFERRED |

20 tasks. Cap respected.

## Dependency order

- T-D01 → T-D02 → T-D03 → T-D04 → T-D05 (sequential; sub-spec scaffolding).
- T-D06..T-D09 parallel (independent workflow edits).
- T-D10 / T-D11 / T-D12 parallel (independent config + docs).
- T-D13 → T-D14 → T-D15 (sequential within `README.md`).
- T-D16, T-D17 after all workflow edits.
- T-D18, T-D19 after every other task.
- T-D20 strictly post-Lane-A merge — not part of this lane's branch.

## Verification per task

- T-D06: `jq '.bomFormat, .specVersion' dist/sbom.cdx.json` returns `"CycloneDX"` + `"1.7"` in the release workflow's CI run.
- T-D07: `grep desktop .github/workflows/actionlint.yml` returns no match on the `runs-on:` line.
- T-D08: `gh api repos/yolo-labz/noctalia-appmenu/code-scanning/alerts` lists qmllint findings post-PR-merge.
- T-D09: job appears in `gh pr checks` output for every PR.
- T-D10: `jq '.rules | length' .github/rulesets/main.json` ≥ 1; required-checks list cardinality matches `contracts/ci-required-checks.md`.
- T-D11: `grep 'sonar.coverage.minimum' sonar-project.properties` returns 65.
- T-D12: `ls docs/adr/ADR-0025-*.md` shows the file.
- T-D13..T-D15: `grep '^## Verify the install$' README.md` + `grep '^## Caveats$' README.md` both match.
- T-D16/T-D17: locally run; CI re-runs them on push.

## Out of scope

- `bridge/src/atspi.rs` refactor — Lane A owns this; Lane D records the waiver only.
- cargo-machete.yml `desktop` host-pin — left as-is per plan.md §Open questions.
- Distribution outside the Nix flake (Homebrew, AUR, etc.).
- New Dependabot triage — parent handles around lane-merge windows.
