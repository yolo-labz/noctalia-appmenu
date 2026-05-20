# Deploy-idempotence checklist — spec 015 SC-004 gate

**Owner:** `scripts/verify-release.sh` gate `deploy-idempotence`
**Hypothesis:** invoking the release skill twice against the
same VERSION is a no-op for every stage that already completed
successfully. This catches the v1.0.17 cycle (cache-nuke wasn't
re-run by hand, so the second invocation needed to restore
freshness).

Each row is a finite pass/fail check.

## Stage idempotence — invocation 1

Run `scripts/release.sh <VERSION>` end-to-end (assume the
plugin bump PR is already merged so `plugin-merge` is a no-op).

- [ ] **DI-001** `preflight` stage passes; clean main worktree.
- [ ] **DI-002** `plugin-tag` stage refuses to re-tag if tag
      already on origin (warn + continue, no error).
- [ ] **DI-003** `plugin-release` stage waits for tag-driven
      release.yml; observes the published release; continues.
- [ ] **DI-004** `nixos-bump` stage detects flake.lock already
      at the target SHA and no-ops.
- [ ] **DI-005** `nixos-deploy` stage runs `nixos-rebuild
      switch` regardless of generation parity (cheap reactivate).
- [ ] **DI-006** `cache-nuke` stage removes `.qmlc` files;
      reports count nuked.
- [ ] **DI-007** `shell-restart` stage restarts the user service.
- [ ] **DI-008** `verify` stage confirms
      `noctalia-appmenu-bridge --version == VERSION` and proxy
      bus name present.

## Stage idempotence — invocation 2 (same VERSION, no changes)

Immediately re-run `scripts/release.sh <VERSION>`.

- [ ] **DI-020** `preflight` passes (tree still clean).
- [ ] **DI-021** `plugin-pr` detects PR is already merged (or
      skipped via `--skip-stage`).
- [ ] **DI-022** `plugin-merge` detects merged state, logs
      `already merged`, returns.
- [ ] **DI-023** `plugin-tag` detects tag on origin, logs
      `tag already on origin`, returns.
- [ ] **DI-024** `plugin-release` detects release present,
      logs `release published`, returns.
- [ ] **DI-025** `nixos-bump` detects flake.lock at target SHA,
      logs `flake.lock already at $TAG`, returns.
- [ ] **DI-026** `nixos-deploy` is allowed to no-op-rebuild
      (faster path; no failures).
- [ ] **DI-027** `cache-nuke` reports `0 .qmlc files` (cache
      was empty from invocation 1; no error).
- [ ] **DI-028** `shell-restart` restarts; no failures.
- [ ] **DI-029** `verify` re-confirms VERSION match.

## Cross-deploy artefact integrity

- [ ] **DI-040** `~/.config/noctalia/plugins/noctalia-appmenu/
      AppmenuPopupWindow.qml` symlinks into the correct Nix
      store path for VERSION. Verified via
      `readlink -f <path> | grep <VERSION>`.
- [ ] **DI-041** `noctalia-appmenu-bridge.service` ExecStart
      references the correct Nix store path. Verified via
      `systemctl --user show noctalia-appmenu-bridge.service
      --property=ExecStart`.

## HM-backup-collision recovery

A known failure mode: HM activation barfs on stale `.backup`
files. The release skill SHALL handle this gracefully.

- [ ] **DI-050** The skill's `nixos-deploy` stage SHALL clear
      `~/.claude/plugins/*.backup` and
      `~/.config/git/hooks/*.backup` before invoking
      `nixos-rebuild switch`. Verifiable by planting a
      `.backup` file pre-run and confirming it's removed.
- [ ] **DI-051** When a NEW `.backup` collision appears that
      the skill does NOT pre-clear (uncommon path), the gate
      fails with `[gate name=deploy-idempotence] FAIL —
      unhandled HM-backup at <path>` and points the operator
      at the remediation.

## Result roll-up

Gate emits one summary line per invocation:
`[gate name=deploy-idempotence invocation=1|2 result=PASS|FAIL
stages_passed=N stages_failed=M]`

PASS condition: invocation 2 stages 020..029 all pass AND
DI-040/041/050 pass.
