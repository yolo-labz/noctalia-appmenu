# Reproducibility gate repair — 21/09/2026

Generator: OpenAI GPT-6 Astra/xhigh. Base: `29d0995`. This is the **gated workflow
slice**, separate from ordinary Sonar configuration/docs PR #225. Coordinator
owns GLM-5.3 independent review and merge; no review/merge is claimed here.
Canonical swarm report: PR #225's `docs/swarm-2026-09-21.md`; spec 017 there.

## Observed cause and minimal change

PR #223 job `106292514146` built/tested successfully, then failed entering the
unrelated devShell to run `cargo clean`: cargo-llvm-cov's `.drv` was not valid.
It never reached a second build or hash comparison. Host Cargo's target cache
is not the Nix sandbox cache. Remove that irrelevant step, retain `result` as a
GC root until the rebuild, keep `nix build --rebuild` and byte comparison, and
use `${{ runner.temp }}` in upload-artifact paths (not literal `$RUNNER_TEMP`).
The broader runner store/GC cause is not repaired or asserted known here.

No required job name, action pin, coverage threshold, permission, source or
exclusion changed. No shared cache clean or GC performed.

## Executed verification

- `python3 scripts/verify-reproducibility.py`: PASS. Executes the workflow's real
  compare script: matching synthetic bytes pass; changed and missing bytes fail.
  Asserts forced rebuild remains and artifact paths use Actions interpolation.
- `nix develop --command bash -c 'actionlint .github/workflows/reproducibility.yml && zizmor --format=plain .github/workflows/reproducibility.yml'`:
  PASS; no findings (two inherited suppressions).
- Actual local build → forced rebuild → SHA-256 comparison, exit **0**:

  ```bash
  nix build .#noctalia-appmenu-bridge --cores 8 --max-jobs 1 --print-build-logs --accept-flake-config
  cp -L result/bin/noctalia-appmenu-bridge "$PI_SCRATCH_DIR/r5-build-1"
  nix build .#noctalia-appmenu-bridge --cores 8 --max-jobs 1 --print-build-logs --accept-flake-config --rebuild
  sha256sum "$PI_SCRATCH_DIR/r5-build-1" result/bin/noctalia-appmenu-bridge
  cmp "$PI_SCRATCH_DIR/r5-build-1" result/bin/noctalia-appmenu-bridge
  ```

  Both hashes: `f8d19779f1ab89e5ff3d804813fc5f0b81a5423e766624b66528289e1c59fedd`.
  Eight cores/one derivation bound this worker while the swarm shared the host.
  The first cold-dependency attempt timed out after 360 seconds; the resumed
  complete run passed. Compact output: `evidence/swarm-2026-09-21/nix-rebuild-retry.txt`.
  This validates same-source local reproducibility, **not** the unmodified #223
  dependency head or its remote runner recovery.

## Gates / reversal

Workflow approval, exact-head different-family review and CI still required.
The workflow branch still inherits the Sonar root defect until #225 lands and
it is updated. No deployment. After approved squash merge, revert via a feature
branch running `git revert <squash-sha>` and open a PR.
