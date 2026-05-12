# Specification: CI + quality gate + documentation (Lane D)

**ID:** 008-ci-quality-docs
**Parent spec:** 004-project-completion
**Lane:** D (last to merge — consumes Lane A's AT-SPI integration fixture)
**Created:** 2026-05-12
**Author:** @phsb5321
**Constitution version:** 1.0.0

## Why

Lane D lands the CI/release/quality-gate/docs work for `v1.0.0`. Each item is a defect or a required-checks gap surfaced in the umbrella audit (`specs/004-project-completion/research.md` §4 + §6). None is speculative; every fix maps to a failing acceptance gate today.

Concretely:

- The `v0.3.0` SBOM is technically nonconforming — `release.yml:77` emits CycloneDX 1.6 while the attestation step claims 1.7 (`research.md` §4, finding 1).
- The AT-SPI substrate (ADR-0024, the basis of every menu render at v1) has zero CI coverage (`research.md` §4, finding 2).
- The required `actionlint + zizmor` check hard-pins to a single `desktop` runner — violating ADR-0013's runner-agnostic-CI invariant (`research.md` §4, finding 3).
- `qmllint` runs exit-code-only against `BarWidget.qml` alone; `AppmenuPopupWindow.qml` is not linted; SARIF is not uploaded (`research.md` §4, FR-024).
- SonarQube quality-gate thresholds are alpha-era (60 % line coverage floor; no new-code gate, no duplication ceiling, no cognitive-complexity hard cap) (`research.md` §6).
- `find_app_for_pid` and `fetch_menu_tree` in `bridge/src/atspi.rs` exceed the cognitive-complexity ceiling of 15 (estimated 18–22 and 16–20 respectively per the Sonar audit). Lane A owns `atspi.rs`; Lane D cannot refactor it directly within this lane's boundary.
- The README still describes the v0.1 substrate (DBusMenu/Registrar) and lacks a working "Verify the install" recipe matching the v1.0.0 substrate (ADR-0024).

This sub-spec consolidates those into one mergeable change set scoped strictly to `.github/workflows/*`, `.github/rulesets/*`, `sonar-project.properties`, `README.md`, and one new ADR (`docs/adr/ADR-0025-cognitive-complexity-waiver.md`). The lane neither edits `bridge/src/**` nor `plugin/**` — those belong to Lanes A and B respectively.

## User scenarios

Reuses umbrella Scenarios 5–7 verbatim (`specs/004-project-completion/spec.md`). Lane D's contribution is the CI plumbing + docs that *make* those scenarios reproducible:

- **Scenario 5 (AT-SPI bus crash + restart):** CI's AT-SPI integration test (FR-022) replays the recovery path on every PR. Lane A delivers the test fixture; Lane D wires the CI job.
- **Scenario 6 (niri reload):** out of scope for this lane; Lane A owns the niri-reconnect coverage.
- **Scenario 7 (release artefact verifies on a fresh box):** Lane D fixes the CycloneDX 1.6 → 1.7 mismatch (FR-021) and adds the `gh attestation verify` dry-run as a required check before tag.

## Functional requirements

### Release workflow (`.github/workflows/release.yml`)

- **FR-021** `release.yml`'s syft invocation emits `cyclonedx-json@1.7=...` (was `@1.6`). The `actions/attest-sbom` step claim "CycloneDX 1.7" now matches the document `specVersion`. Verifiable by parsing the emitted SBOM and asserting `bomFormat = "CycloneDX"` ∧ `specVersion = "1.7"`.

### CI workflow (`.github/workflows/ci.yml`)

- **FR-022** An `AT-SPI integration test` job runs on every PR. It calls `cargo test --test atspi_integration` against the fake-AT-SPI-registry harness Lane A ships at `bridge/tests/atspi_integration.rs`. Until that file lands, the job no-ops with success so Lane D is not blocked on Lane A's merge; once the file lands, the job activates automatically (file-existence gate, no manual cutover).
- **FR-024** A `plugin-lint` step emits `qmllint` SARIF for every `plugin/*.qml` (globbed, so `SubmenuPopup.qml` is auto-included when Lane B lands it) and uploads via `github/codeql-action/upload-sarif`. Findings surface in the GitHub Security tab.

### actionlint workflow (`.github/workflows/actionlint.yml`)

- **FR-023** `runs-on` drops the `desktop` host-pin. New shape: `[self-hosted, Linux, X64, noctalia-appmenu]`. The job runs on any runner registered with the project label. The inline comment is rewritten to document the underlying constraint (`runner-balloon-hook.sh` on vm103 writes `NODE_OPTIONS` to `$GITHUB_ENV`) and points at the upstream remediation (hook fix on vm103) rather than encoding the workaround at the workflow level.

### Repository Ruleset (`.github/rulesets/main.json`)

- **FR-025** The Ruleset on `main` requires every check listed in `specs/004-project-completion/contracts/ci-required-checks.md` before `v1.0.0` is tagged:
  `Lint & format`, `bridge-test`, `plugin-lint`, `reproducibility`, `osv-scanner`, `scorecard`, `codeql`, `SonarQube standalone scan`, `AT-SPI integration test`, `attestation verify (dry-run)`. Ruleset JSON is checked-in as a documented export; the live GitHub config is applied out-of-band by Pedro.

### Quality gate (`sonar-project.properties`)

- **FR-026** Sonar properties express the v1 thresholds: overall line coverage ≥ 65 %, new-code line coverage ≥ 80 %, code duplication < 3 % overall, cognitive complexity ≤ 15 per function, blocker/critical issues = 0. Values that can only be expressed in the SonarQube UI (e.g. new-code gate) are documented in-file as comments pointing at the UI setting.

### Cognitive-complexity waiver (`docs/adr/ADR-0025-cognitive-complexity-waiver.md`)

- **FR-027** `find_app_for_pid` (`bridge/src/atspi.rs:295–408`) and `fetch_menu_tree` (`:618–746`) exceed cognitive complexity 15 today. Lane A owns `atspi.rs` and may refactor them under spec 005; Lane D records a time-boxed waiver via ADR-0025 so the v1.0.0 quality-gate snapshot stays green while Lane A's refactor lands. The ADR expires when Lane A's refactor merges OR at v1.0.1, whichever is sooner.

### Documentation (`README.md`)

- **FR-028** The README contains a "Verify the install" section reproducing `specs/004-project-completion/quickstart.md`, condensed for a fresh-NixOS user. Recipe lists prerequisites (`services.gnome.at-spi2-core.enable = true`, `programs.noctalia.plugins.appmenu.enable = true`), every check the user runs after `nh os switch`, and the expected output of each check. CI executes the automatable subset headlessly via FR-022.
- **FR-029** The README documents the v1 caveats: Firefox needs `accessibility.force_disabled = 0`; Electron apps need `--force-accessibility`; multi-monitor menubar duplication deferred to v2; Alt-letter mnemonics + global Alt-F intercept deferred to v2; GTK4 `GtkPopoverMenuBar` empty-children quirk falls back to `.desktop`-derived pseudo-menu.

## Non-functional requirements

- **NFR-D1 Workflow hygiene.** Every workflow Lane D touches passes `actionlint` and `zizmor` clean. Net-new SHA pins carry a `# vX.Y.Z` trailing comment so Dependabot's regex recognises the entry.
- **NFR-D2 Runner-agnostic.** No workflow Lane D edits hard-pins to a single host label. `desktop` and `vm103` host labels stay valid for jobs that genuinely need a specific runner (e.g. `cargo-machete.yml`) but `actionlint.yml` is no longer one of them.
- **NFR-D3 Required-checks completeness.** The Ruleset's required-checks set matches `contracts/ci-required-checks.md` 1:1. A drift between the two is a planning bug.

## Out of scope (deferred lanes / future work)

- Refactor of `find_app_for_pid` / `fetch_menu_tree` (Lane A — spec 005). Lane D records the waiver; Lane A does the refactor or extends the waiver.
- Dependabot triage of #64, #67, #68, #70, #71 — handled by the parent before/around the lane-D merge; not Lane D's surface.
- New ADRs beyond ADR-0025. The AT-SPI substrate ADR-0024 already governs the integration-test scope.
- noctalia-shell upstream changes (plugin marketplace, manifest spec evolution). Out per `specs/004-project-completion/spec.md` §Out of scope.
- Distribution outside the Nix flake (Homebrew, AUR, etc.). Same.

## Constraints / dependencies

- **Constitution VI** — every GitHub Action pinned by full 40-char SHA + `# vX.Y.Z` trailing comment; never strip the comment.
- **`.github/zizmor.yml`** allowlist already covers all workflows under audit; Lane D may not weaken it.
- **`.github/actionlint.yaml`** registers the custom labels (`noctalia-appmenu`, `desktop`, `vm103`) — kept as-is so per-host labels remain valid for the workflows that legitimately need them.
- **Lane A ordering** — FR-022 ships a file-existence-gated CI job. Until `bridge/tests/atspi_integration.rs` exists on `main`, the job no-ops with success. After Lane A merges, the job activates automatically.
- **Sonar UI thresholds** — new-code coverage gate is set in the SonarQube web UI; the `.properties` file documents the expected value as a comment but cannot enforce it from the repo.

## Success criteria

- **SC-D1** `actionlint -color .github/workflows/` clean on every workflow change.
- **SC-D2** `zizmor --format=plain .github/workflows/` clean on every workflow change.
- **SC-D3** `release.yml`'s emitted SBOM has `bomFormat = "CycloneDX"` and `specVersion = "1.7"`, asserted by a CI step.
- **SC-D4** `AT-SPI integration test` job appears in the CI workflow listing on every PR. Job is green pre-Lane-A (no-op) and post-Lane-A (real cargo test run).
- **SC-D5** `actionlint.yml`'s `runs-on:` line does NOT contain `desktop`.
- **SC-D6** `qmllint` SARIF appears in the GitHub Security tab on every PR that touches `plugin/*.qml`.
- **SC-D7** `sonar-project.properties` reflects the v1 thresholds (line ≥ 65, complexity ≤ 15, duplication < 3 %). New-code 80 % gate documented as a UI-side comment.
- **SC-D8** README contains a `## Verify the install` H2 that reproduces the quickstart recipe and a `## Caveats` H2 (or equivalent) documenting the v1 caveats.
- **SC-D9** `docs/adr/ADR-0025-cognitive-complexity-waiver.md` exists, status = Accepted, expiry tied to Lane A's refactor OR v1.0.1.
- **SC-D10** All Lane D commits are DCO-signed; the branch is pushed to `origin`; **no PR is opened** (parent reviews and opens the PR).

## Key entities

- **release.yml** — release workflow; sole owner of the SBOM emit step.
- **ci.yml** — main CI workflow; sole owner of `bridge-test`, `plugin-lint`, `nix-flake-check`, `commitlint`, and (new at v1) `AT-SPI integration test`.
- **actionlint.yml** — workflow-static-analysis workflow; runner-agnostic at v1.
- **rulesets/main.json** — checked-in export of the live GitHub Ruleset config on `main`.
- **sonar-project.properties** — Sonar scanner config; quality-gate thresholds plus exclusions.
- **README.md** — top-level user-facing entry point; v1's reference install recipe lives here.
- **ADR-0025** — cognitive-complexity waiver covering `find_app_for_pid` + `fetch_menu_tree`; expires when Lane A's refactor merges OR at v1.0.1.

## Assumptions

- Lane A's `bridge/tests/atspi_integration.rs` will land on `main` before `v1.0.0` is tagged. If it slips, FR-022's no-op gate keeps Lane D mergeable and the required-check passes vacuously until the file appears.
- The SonarQube UI's "new-code 80 %" gate is configured out-of-band by Pedro after this lane lands. The properties file documents the expected value; enforcement is server-side.
- The Repository Ruleset is bootstrapped per `~/NixOS/meta/yolo-labz-release-engineering-research.md` §0 (Ruleset over classic protection; `enforcement: disabled` → merge → `active`). Lane D ships the JSON export; Pedro applies it.
- Lane A's complexity-refactor decision (refactor vs. extend the waiver) is made within spec 005; Lane D's waiver covers the worst-case "refactor doesn't land before tag" path.
