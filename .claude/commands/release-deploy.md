---
description: "Canonical end-to-end release + deploy flow for noctalia-appmenu (plugin merge → tag-on-correct-commit → NixOS flake bump → nixos-rebuild → shell restart → version verify). Idempotent."
---

# /release-deploy VERSION

Codifies the lessons from v1.0.5..v1.0.17 — fifteen tag iterations across
one popup-dismiss bug, two of which (v1.0.14, v1.0.16) tagged the wrong
commit and a third (v1.0.10) shipped a binary that hadn't restarted on
the host. This skill is **the** way to ship a noctalia-appmenu release.

## Prerequisite

The bridge `Cargo.toml` version bump and any plugin/spec edits MUST already
have an open PR via the canonical worktree-first workflow:

```bash
NN=$(gh pr list --state all --limit 1 --json number -q '.[0].number + 1')
git worktree add ../noctalia-appmenu-${NN}-bump -b ${NN}-bump origin/main
cd ../noctalia-appmenu-${NN}-bump
# bump bridge/Cargo.toml; cargo check; commit; push; gh pr create
```

The PR title MUST contain `vX.Y.Z` literally — that's how the script finds it.

## What this skill does

Runs `scripts/release.sh VERSION` from the **main** worktree. Each stage is
idempotent; re-running after a partial failure resumes at the first
incomplete stage.

| Stage | What | Why it's load-bearing |
|---|---|---|
| `preflight` | Clean main worktree, version present in Cargo.toml, NixOS root exists, gh authed | Stops drift before any side effect |
| `plugin-pr` | Finds open PR with `VERSION` in title, watches checks to green | One human review pass on bump |
| `plugin-merge` | `gh pr merge --squash --delete-branch` | Single canonical commit on main |
| `plugin-tag` | **Refuses to tag** unless `origin/main` HEAD subject contains `VERSION` | Catches the v1.0.14 drift mechanically (trigger E in CLAUDE.md) |
| `plugin-release` | Polls `gh release view vVERSION` until artifacts published | Confirms tag-driven release.yml fired |
| `nixos-bump` | Worktree on `phsb5321/NixOS`, `nix flake lock --update-input noctalia-appmenu`, eval desktop, push PR, admin-merge | The user-CLAUDE.md forbids `git push origin main`; PR + admin-merge is the only safe path |
| `nixos-deploy` | `sudo nixos-rebuild switch --flake .#$(hostname)` (clears stale HM `.backup` first) | `nh os switch` was banned by a niri-block hook on this host |
| `cache-nuke` | `rm -rf ~/.cache/noctalia-qs/qmlcache/` | Nix store sets all file mtimes to the 1969-12-31 epoch; Quickshell's mtime-based `.qmlc` freshness check then prefers the WALL-CLOCK-dated cache from the previous release over the new source. Symptom: Pedro screenshots labelled "absolutely nothing changes !!" three releases running (v1.0.15, v1.0.16, v1.0.17). Cache must die BEFORE the shell restart, or the restart re-loads yesterday's bytecode. |
| `shell-restart` | `systemctl --user restart noctalia-shell.service` | Memory `feedback_nh_switch_no_shell_restart.md` — switch alone does NOT reload user services |
| `verify` | `noctalia-appmenu-bridge --version == VERSION` and `org.noctalia.AppMenu` present on session bus | Catches "deploy claimed, binary unchanged" (drift trigger G) |

## Invocation

```bash
scripts/release.sh 1.0.18                       # full flow
scripts/release.sh 1.0.18 --skip-stage plugin-merge  # PR already merged
scripts/release.sh 1.0.18 \
    --skip-stage plugin-merge \
    --skip-stage plugin-tag                     # resume from nixos-bump
```

## Hard bans honoured by the script

1. No `git push origin main` — NixOS bump is PR + admin-merge only.
2. No `git stash` anywhere; uses worktrees for the NixOS bump.
3. No `--no-verify`; pre-commit + commit-msg hooks run.
4. No tag-pin without SHA comment (workflow files are not touched).
5. No `git add -A`; only `git add flake.lock` for the NixOS bump.
6. Never re-tag a release — preflight aborts if `vVERSION` already on origin.

## When NOT to use

- **Hotfix that won't be tagged** — use the normal PR workflow; do not invoke this.
- **Out-of-band Cargo bump** — the script demands a PR. Open one first.
- **Multi-host fan-out** — script deploys the current host only. Add SSH-deploy
  to siblings as a separate task; do not extend this script (single
  responsibility — drift trigger H).

## Recovery from script failure

The script is `set -euo pipefail`. On error it prints the failing stage's
name. Re-run with `--skip-stage` for every stage already completed; the
preflight / idempotency checks in each remaining stage will detect prior
progress and no-op where appropriate.

## Why this is a skill, not a Makefile

This document IS the audit trail. Future Claude sessions debugging a v1.0.NN
hotfix will read the file → know the canonical recipe → run it → not invent
a parallel one. The drift case study in `CLAUDE.md` is the negative example;
this skill is the positive one.
