# Multi-worker dispatch — 004-project-completion

**Author:** parent Claude Code session (spec 004 PR #73)
**Date:** 2026-05-12
**Pattern source:** `~/Documents/Code/CLAUDE.md` §Multi-agent orchestration (proven 23/04/2026, spec 016 portfolio refresh).

This document is the parent-side runbook for dispatching the four lane workers. Each lane has its own self-contained brief under `briefs/lane-{a,b,c,d}.md`. The briefs are what the parent shells out to `claude --print` as the prompt.

## Pre-flight checklist (before firing ANY worker)

- [x] Spec 004 PR (#73) merged OR sign-off in PR body that workers may proceed against the as-yet-unmerged branch
- [x] Dependabot triage done (PRs #65/#66/#69/#70/#72 merged or queued, #71/#67/#68/#64 deferred with comments)
- [x] Worktree `noctalia-appmenu-004-project-completion` exists and is current with `origin/004-project-completion`
- [ ] Latest `origin/main` SHA recorded in `briefs/_dispatch.md §Provenance` below
- [ ] Worker log directory exists: `/tmp/spec-004-workers/`
- [ ] Anthropic credit budget confirmed (≤ $28 total for all four lanes)

## Branch numbering

Latest PR # at dispatch time: **#73** (spec 004 itself, currently open). Next four feature branches:

| Lane | Branch | Source-of-truth |
|---|---|---|
| A | `74-bridge-completion` | `briefs/lane-a-bridge.md` |
| B | `75-plugin-completion` | `briefs/lane-b-plugin.md` |
| C | `76-nix-completion` | `briefs/lane-c-nix.md` |
| D | `77-ci-quality-docs` | `briefs/lane-d-ci-quality-docs.md` |

> If Dependabot auto-merges land before dispatch and consume PR numbers, the dispatcher MUST recompute next-PR-# via `gh pr list --state all --limit 1 --json number -q '.[].number'` and update each brief's "Your branch" section before invocation.

## Canonical dispatch command (per worker)

```bash
LANE=a   # or b / c / d
SLUG=bridge-completion   # match the table above
NN=74    # match the table above
BRIEF="$HOME/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/briefs/lane-${LANE}-${SLUG}.md"
LOG="/tmp/spec-004-workers/lane-${LANE}.jsonl"

mkdir -p "$(dirname "$LOG")"

claude --print \
  --model "claude-opus-4-7[1m]" \
  --max-turns 100 \
  --max-budget-usd 10 \
  --permission-mode acceptEdits \
  --output-format stream-json \
  --include-partial-messages \
  --verbose \
  --add-dir "$HOME/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion" \
  --add-dir "$HOME/Documents/Code/yolo-labz/noctalia-appmenu" \
  --allowedTools "Read Edit Write Glob Grep TodoWrite Bash(cargo *) Bash(nix *) Bash(qmllint *) Bash(git status:*) Bash(git diff:*) Bash(git log:*) Bash(git add:*) Bash(git commit:*) Bash(git push:*) Bash(git fetch:*) Bash(git rebase:*) Bash(git worktree:*) Bash(git rev-parse:*) Bash(git branch:*) Bash(gh pr list:*) Bash(gh pr view:*) Bash(gh pr checks:*) Bash(ls *) Bash(mkdir *) Bash(find *) Bash(test *) Bash(stat *) Bash(file *) Bash(rm *.tmp) Bash(rm -f *.tmp)" \
  --name "spec-004-lane-${LANE}" \
  "$(cat "$BRIEF")" \
  > "$LOG" 2>&1 &
```

> Run in background (`&`). Capture `session_id` from first stream event: `grep -m1 session_id "$LOG" | jq -r '.session_id'`. Resume any worker via `claude --resume <session_id>`. Final result: `grep '"type":"result"' "$LOG" | tail -1`.

## Dispatch order

Per `tasks.md §Implementation strategy`:

1. **First**: Lane A alone (`tasks.md` T006). Runs ~10–15 min. Parent watches for first commit, then proceeds.
2. **After Lane A merges**: Lanes B + C in parallel (T007 + T010 + T012). Parent reviews + opens PR for each.
3. **After Lanes B + C merge**: Lane D last (T020–T023). Consumes Lane A's AT-SPI integration test.

> The plan permits firing all 4 in parallel, but `tasks.md §Implementation strategy` recommends sequential MVP first. Parent's call. The DIY shell pattern's anti-pattern threshold is 5+ children — staying at 4 is safe.

## Hard constraints on every worker (re-stated in each brief)

1. **Worktree-first.** Each worker creates its own `../noctalia-appmenu-NN-slug/` worktree via `git worktree add ... -b NN-slug origin/main`. Never edits in the main worktree.
2. **No push to `main`.** Workers push only to their feature branch.
3. **No PR creation.** Workers report a "ready for PR" status; the parent opens the PR after review.
4. **DCO + Conventional Commits.** Lefthook enforces. Use `git commit -s -m "type(scope): ..."`.
5. **Test before commit.** `cargo test` / `qmllint` / `nix flake check` per lane.
6. **Stop at budget.** `--max-budget-usd 10` is a hard cap. Worker returns whatever's done if hit.
7. **Speckit chain.** Worker runs its own `specify → plan → tasks → implement` cycle inside its child sub-spec.

## Provenance

| Item | Value |
|---|---|
| Spec PR | https://github.com/yolo-labz/noctalia-appmenu/pull/73 |
| Origin/main SHA at dispatch | (record `git rev-parse origin/main` here right before firing) |
| Parent session model | claude-opus-4-7[1m] |
| Worker model | claude-opus-4-7[1m] |
| Worker permission mode | acceptEdits (NOT bypassPermissions) |
| Total budget ceiling | $28 across 4 lanes ($10/$6/$5/$7) |

## Failure modes + mitigations

- **Worker exceeds 100 turns w/o commit** → parent kills with `kill %1` (or `KillShell`); investigates log.
- **Worker pushes a broken branch** → parent does not open the PR; deletes the branch, refines brief, re-dispatches.
- **Two workers race on shared lockfiles** → file-collision-free design per `plan.md §Architecture sketch` precludes this. If it happens, parent reconciles manually.
- **Worker takes a destructive action** (e.g. unlinked `git push -f`) → allowlist excludes `git push:*` past origin's branch; safety relies on the worker not invoking out-of-allowlist verbs.

## Post-dispatch parent obligations

- Watch each worker log via `tail -f /tmp/spec-004-workers/lane-X.jsonl | jq -c 'select(.type=="message" or .type=="result")'`
- On worker completion: read final summary, run `git log origin/main..` on the worker's branch, review diff, open PR with body referencing spec 004 + the relevant FRs.
- Merge in dependency order per `tasks.md §Implementation strategy`.

> See `tasks.md` for the per-task ordering and `analyze.md` for the open scope questions reviewers should resolve in the spec PR before dispatch fires.
