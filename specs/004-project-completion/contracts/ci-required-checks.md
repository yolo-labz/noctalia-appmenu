# Contract: CI required-status-checks set (Lane D)

**Status:** modified at v1.0.0
**File:** `.github/rulesets/main.json` (export of live GitHub config)
**Owner:** Repository Ruleset on `main`

## Required checks at v1.0.0 tag

The Repository Ruleset MUST require every check below to be green on the merge commit before `main` is tagged `v1.0.0`. Order is informational — GitHub enforces them as a set.

| # | Check name | Workflow | Job/Step | Notes |
|---|---|---|---|---|
| 1 | `Lint & format` | `ci.yml` | `lint` | rustfmt, clippy, alejandra. Existing. |
| 2 | `bridge-test` | `ci.yml` | `bridge-test` | `cargo test --all-features --locked`. Existing. |
| 3 | `plugin-lint` | `ci.yml` | `plugin-lint` | qmllint over all `plugin/*.qml`; SARIF upload (FR-024). |
| 4 | `reproducibility` | `ci.yml` (or `reproducibility.yml`) | `reproducibility` | Builds the bridge twice; diffs the binary. FR-019. |
| 5 | `osv-scanner` | `osv-scanner.yml` | `osv-scanner` | Existing. |
| 6 | `scorecard` | `scorecard.yml` | `scorecard` | Existing (push-only — kept as required for tag-cuts; non-blocking on PRs). |
| 7 | `codeql` | `codeql.yml` | `codeql` matrix | Rust + JavaScript matrices (the latter for any embedded TS in workflows). |
| 8 | `SonarQube standalone scan` | `sonar.yml` | `sonar-scan` | New at v1.0.0 as a required check (was advisory). |
| 9 | `AT-SPI integration test` | `ci.yml` | `atspi-integration` | New at v1.0.0. Runs the fake-AT-SPI-registry harness from Lane A (FR-022). |
| 10 | `attestation verify (dry-run)` | `release.yml` (or dedicated workflow) | `attestation-dry-run` | Validates `gh attestation verify` against the candidate artefact before tagging. |

## Behaviour under runner outage (FR-023)

- Required checks must NOT hard-pin to a single runner label (`desktop`). All workflows use the runner-agnostic label set `[self-hosted, Linux, X64, noctalia-appmenu]`.
- If `vm103` (the current self-hosted runner) is offline, runs queue against any other runner with the `noctalia-appmenu` label. If none is available, jobs queue indefinitely; the Repository Ruleset behaviour is "wait, not skip" — this is acceptable per ADR-0013.

## Behaviour for Dependabot PRs

- Dependabot PRs MUST satisfy the same required-checks set. No bypass.
- Major-version bumps (e.g. #68 codecov v6, #64 deploy-pages v5) require an additional manual review step before merge — the ruleset does NOT auto-merge.

## Sonar quality gate (FR-026)

Configured in the SonarQube server UI for project `yolo-labz_noctalia-appmenu`:

| Metric | Threshold |
|---|---|
| Overall line coverage | ≥ 65 % |
| New-code line coverage | ≥ 80 % |
| Code duplication (overall) | < 3 % |
| Cognitive complexity per function | ≤ 15 |
| Blocker issues | = 0 |
| Critical issues | = 0 |
| Security hotspots reviewed | 100 % on new code |

If the threshold cannot be met for a documented reason, an ADR captures the deviation (e.g. ADR-0025 would document a cognitive-complexity waiver for `find_app_for_pid` / `fetch_menu_tree`).

## Test contract

- Lane D's child spec ships a `.github/actionlint.yml`-validated workflow diff.
- The release workflow's CycloneDX format is verified by an explicit step that parses the emitted SBOM and asserts `bomFormat = "CycloneDX"` + `specVersion = "1.7"` (FR-021).
- The `attestation verify (dry-run)` check exercises `gh attestation verify` against the candidate artefact; failure = block merge.

## Non-goals

- This contract does NOT require a packaging job (Homebrew, AUR, etc.). Distribution outside the Nix flake is post-v1 work.
- This contract does NOT require Windows / macOS runners. The project is Linux-only by constitution.
