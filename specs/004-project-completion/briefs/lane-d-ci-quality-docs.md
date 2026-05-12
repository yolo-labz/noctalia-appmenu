# Lane D worker brief — `008-ci-quality-docs`

You are a focused claude-code worker assigned **Lane D** of the `noctalia-appmenu` v1.0.0 roadmap. This lane lands LAST — it consumes Lane A's AT-SPI integration test fixture.

## Mission (one paragraph)

Land the CI + quality gate + documentation work for v1.0.0 per the umbrella spec `004-project-completion`. Specifically: fix the CycloneDX 1.6 → 1.7 mismatch in the release workflow (the v0.3.0 SBOM is technically nonconforming), add an AT-SPI integration test job that exercises Lane A's fake-AT-SPI-registry harness, make `actionlint.yml`'s `runs-on` runner-agnostic (drop the `desktop` label hard-pin), wire qmllint SARIF emit + upload, raise the SonarQube quality gate per the audited thresholds, refactor the two cognitive-complexity hot paths in `bridge/src/atspi.rs` OR document a waiver via ADR-0025, and write the README "Verify the install" recipe (FR-028) inheriting from `quickstart.md`. Implement under your own sub-spec at `specs/008-ci-quality-docs/`.

## Source of truth (read in this order, all paths absolute)

1. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/spec.md` — read §User scenarios 5–7, §Functional requirements §CI/release + §Quality gate + §Documentation, §Constraints, §SCs
2. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/plan.md` — §Approach + §Affected files §Lane D + §Risks R4
3. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/research.md` — §4 (CI/release audit) + §6 (Sonar audit)
4. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/contracts/ci-required-checks.md` — required-checks set
5. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/quickstart.md` — full "Verify the install" recipe (your README section inherits from this)
6. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/docs/adr/ADR-0012-self-hosted-runner-only.md`, `ADR-0013-runner-agnostic-ci.md`, `ADR-0014-local-first-ci.md`
7. `/home/notroot/NixOS/meta/yolo-labz-release-engineering-research.md` (constitution VI's reference) — read §0 and §3 (Rust) — non-negotiable supply-chain rules
8. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/.github/workflows/` (current state at v0.3.0 final)

## Your worktree

```bash
cd ~/Documents/Code/yolo-labz/noctalia-appmenu
git fetch origin main
git worktree add ../noctalia-appmenu-77-ci-quality-docs -b 77-ci-quality-docs origin/main
cd ../noctalia-appmenu-77-ci-quality-docs
```

> Confirm Lane A is merged before starting your work. If not yet merged, you can still draft `release.yml`, `actionlint.yml`, qmllint SARIF, Sonar properties, and the README; defer the AT-SPI integration job (FR-022) to a follow-up commit that lands after Lane A.

## Your branch

`77-ci-quality-docs` off `origin/main`.

## FRs assigned to you

- **FR-021** CycloneDX 1.6 → 1.7 in `.github/workflows/release.yml:77`; attestation step claim now matches
- **FR-022** AT-SPI integration test job in `.github/workflows/ci.yml` (consumes Lane A's `bridge/tests/atspi_integration.rs`)
- **FR-023** `actionlint.yml` runner-agnostic — drop `desktop` label hard-pin
- **FR-024** qmllint SARIF emit + upload via `github/codeql-action/upload-sarif`; covers all `plugin/*.qml`
- **FR-025** Repository Ruleset required-checks list per `contracts/ci-required-checks.md`
- **FR-026** Sonar quality gate config — coverage 65% overall + 80% new code, duplication <3%, complexity ≤15, blockers = 0
- **FR-027** refactor `find_app_for_pid` + `fetch_menu_tree` in `bridge/src/atspi.rs` below complexity 15 OR write `docs/adr/ADR-0025-cognitive-complexity-waiver.md`
- **FR-028** README "Verify the install" section inheriting `quickstart.md`
- **FR-029** Documented caveats in README (Firefox `accessibility.force_disabled=0`, Electron `--force-accessibility`, multi-monitor / Alt-key v2 deferrals)

## Your speckit chain

```
specs/008-ci-quality-docs/{spec.md, plan.md, tasks.md, checklists/requirements.md}
```

## Hard constraints

1. **Worktree-first.**
2. **DCO sign-off + conventional commits.** `ci(scope): ...`, `chore(scope): ...`, `docs(scope): ...`.
3. **No push to `main`. No PR creation.**
4. **Action pins.** Every GitHub Action MUST be pinned by full 40-char SHA with trailing `# vX.Y.Z` comment. Constitution VI invariant. Never strip the version comment — Dependabot regex needs it.
5. **Workflow-level `permissions: {}`** (deny-all) with per-job re-grants. Signing jobs need `id-token: write` + `attestations: write` + `contents: read`. `contents: write` only if cutting GitHub Release in same job.
6. **`step-security/harden-runner@<sha>`** in `egress-policy: audit` on every release workflow.
7. **`zizmor`** + **`actionlint`** clean on every workflow change.
8. **`SOURCE_DATE_EPOCH = $(git log -1 --format=%ct)`** for archives.
9. **No `USER_TOKEN`** in CI for SonarQube — only `PROJECT_ANALYSIS_TOKEN`.
10. **No re-tag** (already restated in spec).

## Allowlist of Bash commands

- `actionlint`, `zizmor`, `qmllint`, `nix flake check`, `cargo *` (read-only access to Lane A's tests)
- `git status` / `git diff` / `git log` / `git add` / `git commit` / `git push` (your branch only) / `git fetch` / `git rebase` / `git worktree` / `git rev-parse` / `git branch`
- `gh pr list` / `gh pr view` / `gh pr checks` (NEVER `gh pr create`, NEVER `gh pr merge`)
- `ls`, `mkdir`, `find`, `test`, `stat`, `file`

## Acceptance gates

- [ ] `actionlint` + `zizmor` clean on every workflow file
- [ ] `release.yml` emits CycloneDX 1.7 (`syft . -o cyclonedx-json@1.7=sbom.cdx.json`)
- [ ] AT-SPI integration job present (depends on Lane A's `bridge/tests/atspi_integration.rs`)
- [ ] `actionlint.yml` `runs-on` does NOT contain `desktop`
- [ ] qmllint SARIF uploaded via `github/codeql-action/upload-sarif`
- [ ] `sonar-project.properties` reflects the v1 thresholds
- [ ] README contains a "Verify the install" section reproducing `quickstart.md`
- [ ] All commits DCO-signed
- [ ] Branch pushed

## Reporting

```
LANE D — ci-quality-docs: READY FOR PR
Branch: 77-ci-quality-docs
Commits: <N>
Last commit SHA: <sha>
Sub-spec dir: specs/008-ci-quality-docs/
Acceptance: <PASS/FAIL with one-line rationale>
Open items for PR review: <list>
```

## Anti-patterns

- ❌ Tag-pinning any GitHub Action (full SHA + `# vX.Y.Z` only).
- ❌ Stripping the `# vX.Y.Z` comment from a SHA-pinned action.
- ❌ Re-tagging any release.
- ❌ Editing `CHANGELOG.md` by hand (`git-cliff` owns it).
- ❌ Using `USER_TOKEN` credentials for SonarQube.
- ❌ Bumping codecov-action from v5 to v6 (#68 Dependabot is deferred to post-v1).
- ❌ Adding Sonar quality-gate thresholds that contradict `contracts/ci-required-checks.md` (single source-of-truth).
