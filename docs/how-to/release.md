# Releasing

End-to-end release + deploy is codified in `scripts/release.sh` and
invoked via the `release-deploy` skill. **Don't improvise** — the
script enforces the lessons from v1.0.5..v1.0.17 (15 tag iterations
on one popup-dismiss bug, two of which tagged the wrong commit and a
third shipped a binary that never restarted on the host).

## Prerequisites

- Repository ruleset is in `active` enforcement, not the `disabled`
  bootstrap mode.
- A bump PR is **already open** with the new bridge `Cargo.toml`
  version. The PR title MUST contain `vX.Y.Z` literally — that's
  how the script finds it. Canonical worktree-first bump:

  ```bash
  NN=$(gh pr list --state all --limit 1 --json number -q '.[0].number + 1')
  git worktree add ../noctalia-appmenu-${NN}-bump -b ${NN}-bump origin/main
  cd ../noctalia-appmenu-${NN}-bump
  # edit bridge/Cargo.toml version, cargo check, commit, push, gh pr create
  ```

## Run the canonical flow

```bash
cd ~/Documents/Code/yolo-labz/noctalia-appmenu       # main worktree
scripts/release.sh 1.0.26
```

The script runs 10 idempotent stages; each one is safe to re-run.
Re-invoke with `--skip-stage <name>` for any stage already completed
during a partial run.

| Stage | What it does |
|---|---|
| `preflight` | Clean main worktree, version present in Cargo.toml, NixOS flake root exists, `gh` authed. |
| `plugin-pr` | Finds the open PR whose title contains the version; watches checks to green. |
| `plugin-merge` | `gh pr merge --squash --delete-branch`. |
| `plugin-tag` | **Refuses to tag** unless `origin/main` HEAD subject contains the version. Catches the v1.0.14 wrong-commit drift mechanically. |
| `plugin-release` | Polls `gh release view vVERSION` until artifacts are published. |
| `nixos-bump` | Worktree on `phsb5321/NixOS`, `nix flake lock --update-input noctalia-appmenu`, eval host, push PR, admin-merge. |
| `nixos-deploy` | `sudo nixos-rebuild switch --flake .#$(hostname)` (clears stale HM `.backup` first). |
| `cache-nuke` | `rm -rf ~/.cache/noctalia-qs/qmlcache/` — Nix store mtimes are the epoch (1969-12-31); without this Quickshell's `.qmlc` freshness check re-loads yesterday's bytecode. |
| `shell-restart` | `systemctl --user restart noctalia-shell.service` — `nixos-rebuild` does NOT restart user services. |
| `verify` | `noctalia-appmenu-bridge --version == VERSION` and `org.noctalia.AppMenu` present on the session bus. |

## The release workflow itself

`scripts/release.sh plugin-tag` pushes `vVERSION`. The push triggers
`.github/workflows/release.yml`:

1. `step-security/harden-runner` (egress: audit).
2. `nix build .#noctalia-appmenu-bridge` (deterministic).
3. Plugin tarball with reproducible `tar` flags.
4. SBOMs via `syft` (CycloneDX 1.6 + SPDX 2.3) — note CycloneDX 1.6,
   not 1.7, per [ADR-0026](../adr/ADR-0026-cyclonedx-1.6-syft-constraint.md).
5. `dist/checksums.txt`.
6. `actions/attest-build-provenance@v4.1.0` against the checksums.
7. `actions/attest-sbom@v4.1.0` once per SBOM format.
8. `git-cliff` regenerates `CHANGELOG.md` (ships as Release notes body).
9. GitHub Release with all artefacts attached.

## End-user verification

```bash
gh attestation verify ./noctalia-appmenu-bridge --owner yolo-labz
```

Covered also by [SECURITY.md](../../SECURITY.md).

## If the release botched

**Don't re-tag.** Cut `vX.Y.Z+1`. The yolo-labz invariant is
non-negotiable — `slsa-verifier` validates against the commit SHA at
signing time; re-tagging produces stale provenance.

```bash
# Fix the underlying issue, open a fresh bump PR with the next
# patch number in the title, then re-run:
scripts/release.sh 1.0.27
```

## Hand-edits to CHANGELOG.md

Forbidden. `git-cliff` owns the file (constitution principle VI).
Edit commits, not the changelog. PR #140 (24/05/2026) backfilled
v1.0.0..v1.0.25 via `git-cliff -o CHANGELOG.md` after the file
drifted 25 versions out of date — that regeneration path is the only
sanctioned hand-touch.

## Pre-release tags

`v1.0.0-rc.1`, `v1.0.0-beta.1`, etc. The release workflow's preflight
gate classifies these as prereleases and silently skips secrets it
would otherwise loud-fail on. Useful for dry-running an end-to-end
attestation flow before real users land.
