# Requirements checklist: 008-ci-quality-docs

**Spec:** `specs/008-ci-quality-docs/spec.md`
**Constitution version:** 1.0.0
**Generated:** 2026-05-12

Each FR has a verification cue. Acceptance happens when every box is ticked on the merge commit.

## Functional requirements

- [ ] **FR-021** `release.yml`'s syft step emits `cyclonedx-json@1.7=dist/sbom.cdx.json`. Verification: `jq '.bomFormat, .specVersion' dist/sbom.cdx.json` returns `"CycloneDX"` + `"1.7"` in the release workflow's CI run. Attestation step claim ("CycloneDX 1.7") matches the document `specVersion`.
- [ ] **FR-022** An `AT-SPI integration test` job is present in `ci.yml`. File-existence gate: if `bridge/tests/atspi_integration.rs` is absent, the job emits `notice` and exits 0; if present, it runs `cargo test --test atspi_integration`. Job appears in every PR's `gh pr checks` output.
- [ ] **FR-023** `actionlint.yml`'s `runs-on:` does NOT contain `desktop`. New value: `[self-hosted, Linux, X64, noctalia-appmenu]`. Inline comment rewritten to point at the upstream runner-hook fix.
- [ ] **FR-024** `qmllint` runs over every `plugin/*.qml` file (globbed); SARIF is emitted and uploaded via `github/codeql-action/upload-sarif@<sha> # v4.35.4`. Findings appear in the GitHub Security tab.
- [ ] **FR-025** `.github/rulesets/main.json` exists, exports the required-checks set per `contracts/ci-required-checks.md` 1:1.
- [ ] **FR-026** `sonar-project.properties` reflects v1 thresholds: `sonar.coverage.minimum=65`, cognitive complexity ≤ 15 (already correct), in-file comments document the SonarQube-UI-only thresholds (new-code 80 %, duplication < 3 %, blockers = 0).
- [ ] **FR-027** `docs/adr/ADR-0025-cognitive-complexity-waiver.md` exists, status = Accepted, names `find_app_for_pid` and `fetch_menu_tree`, expiry trigger documented (Lane A refactor merge OR v1.0.1 — whichever first).
- [ ] **FR-028** `README.md` contains a `## Verify the install` H2 that inherits from `quickstart.md`. Recipe lists `services.gnome.at-spi2-core.enable = true`, HM enable, `nh os switch` step, and every check (busctl, journalctl, attestation verify) with expected output.
- [ ] **FR-029** `README.md` contains a `## Caveats` section (or equivalent under "Compatibility") documenting Firefox `accessibility.force_disabled = 0`, Electron `--force-accessibility`, multi-monitor deferred to v2, Alt-key v2 deferral, GTK4 popover quirk.

## Non-functional requirements

- [ ] **NFR-D1** `actionlint -color` clean on every workflow file Lane D touches (`release.yml`, `ci.yml`, `actionlint.yml`). `zizmor --format=plain .github/workflows/` clean. All SHA pins carry `# vX.Y.Z` trailing comments.
- [ ] **NFR-D2** No workflow Lane D edits hard-pins to a single host label.
- [ ] **NFR-D3** Required-checks set in `rulesets/main.json` matches `contracts/ci-required-checks.md` 1:1.

## Process gates

- [ ] All commits DCO-signed (`git log --show-signature` or trailer-grep for `Signed-off-by:`).
- [ ] Conventional-commit subjects (`ci(scope):`, `docs(scope):`, `chore(scope):`).
- [ ] Branch pushed to `origin`; no PR opened by the worker — parent orchestrator opens the PR.
- [ ] Worktree confined to `../noctalia-appmenu-77-ci-quality-docs/`; main worktree untouched.

## Out-of-scope guards

- [ ] No edit to `bridge/src/**` (Lane A boundary).
- [ ] No edit to `plugin/**/*.qml` (Lane B boundary).
- [ ] No edit to `nix/**` or `flake.nix` (Lane C boundary).
- [ ] No edit to `CHANGELOG.md` (`git-cliff` owns it).
- [ ] No re-tag of any release.
- [ ] No GitHub Action pinned by tag — every pin is full 40-char SHA + `# vX.Y.Z` trailing comment.

## Definition of done

All FR + NFR + process boxes ticked. SC-D1..SC-D10 from `spec.md` §Success criteria all evidenceable from `main`'s state at the lane's merge commit.
