# Lane A worker brief — `005-bridge-completion`

You are a focused claude-code worker assigned **Lane A** of the `noctalia-appmenu` v1.0.0 roadmap. The parent has handed you a complete specification chain. Your job is to land the bridge-side work (`FR-001..FR-009`) on a clean feature branch, then stop and report.

You have **never seen this conversation before**. Read the source-of-truth files in the order listed, then proceed.

## Mission (one paragraph)

Land the bridge (`bridge/src/*.rs`) work for v1.0.0 of `noctalia-appmenu` per the umbrella spec `004-project-completion`. Specifically: extract a `FocusSink` trait, fix the niri-IPC backoff regrowth, plug the AT-SPI walker gaps (GTK4 empty-children fallback, persistent connection, `IsEnabled` PropertiesChanged monitor, click re-fetch race), add unit tests for the KDE double-prefix + Anki subprocess-launcher PID match, and delete the retired `dbusmenu.rs` + `registrar.rs` modules. Implement under your own speckit sub-spec at `specs/005-bridge-completion/` (specify → plan → tasks → implement). Open no PR — push the branch and report.

## Source of truth (read in this order, all paths absolute)

1. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/spec.md` — read §Why, §User scenarios 1–6, §Functional requirements §Bridge focus tracker + §Bridge AT-SPI walker, §NFRs, §Constraints, §Success criteria, §Key entities
2. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/plan.md` — §Approach + §Constitution Check + §Affected files §Lane A
3. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/research.md` — §1 (niri focus tracker), §2 (AT-SPI walker) — these are the audit findings you are fixing
4. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/contracts/focus-sink-trait.md` — interface, contract guarantees, test contract
5. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/contracts/active-json-schema.md` — `v=1.1` schema with new `source` field
6. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/.specify/memory/constitution.md` — load-bearing rules (principles I, II, V, VI, VII are most relevant to you)
7. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/docs/adr/ADR-0009-debouncing-policy.md`, `ADR-0016-niri-event-stream-schema.md`, `ADR-0022-bridge-owns-registrar.md`, `ADR-0023-dbusmenu-fetch-on-focus.md`, `ADR-0024-atspi-substrate.md`

## Your worktree

```bash
cd ~/Documents/Code/yolo-labz/noctalia-appmenu
git fetch origin main
git worktree add ../noctalia-appmenu-74-bridge-completion -b 74-bridge-completion origin/main
cd ../noctalia-appmenu-74-bridge-completion
git log origin/main..HEAD --oneline   # MUST be empty
```

All your code edits live in this worktree. Never touch `~/Documents/Code/yolo-labz/noctalia-appmenu/` (main worktree) or `~/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/` (parent's worktree).

## Your branch

`74-bridge-completion` off `origin/main`.

> If `gh pr list --state all --limit 1 --json number -q '.[].number'` returns a number > 73, increment your branch number accordingly (e.g. `75-bridge-completion`). Update plan.md / tasks.md cross-references in your sub-spec.

## FRs assigned to you (all 9)

- **FR-001** backoff reset to floor after ≥30 s connected session — `bridge/src/niri.rs`
- **FR-002** integration test for `run_once` ack-path — `bridge/tests/niri_reconnect.rs` (new)
- **FR-003** extract `FocusSink` trait + `FocusEvent` + `FocusOp` — new `bridge/src/focus.rs`; `niri.rs` becomes one implementor
- **FR-004** GTK4 `GtkPopoverMenuBar` empty-children → synthetic fallback — `bridge/src/atspi.rs`
- **FR-005** subscribe to `org.a11y.Status` PropertiesChanged; re-flip `IsEnabled` on bus restart — `bridge/src/atspi.rs`
- **FR-006** persistent a11y connection (tokio task) — `bridge/src/atspi.rs`
- **FR-007** click forwarding re-fetches before `DoAction(0)`; typed `MenuError::Stale` on path-not-found — `bridge/src/atspi.rs`
- **FR-008** unit tests for `normalize_app_id` (KDE double-prefix + Anki subprocess launcher) — `bridge/src/atspi.rs` (test module)
- **FR-009** delete `bridge/src/dbusmenu.rs` + `bridge/src/registrar.rs`; remove `mod` declarations from `lib.rs` and unused imports

## Your speckit chain

Run the speckit chain inside your own sub-spec dir:

```bash
mkdir -p specs/005-bridge-completion/checklists
# spec.md: derived from spec 004 FRs 001-009 (you may inline or summarise; cite spec 004 §FRs)
# plan.md: tech approach + constitution check (Lane A scope only)
# tasks.md: ≤25 tasks (this is your hard cap)
# implement: write code, write tests, commit per task
```

The umbrella spec already did the requirements work. You may make the sub-spec terse (1-page spec.md, 1-page plan.md, 25-line tasks.md) — the goal is traceability, not duplication.

## Hard constraints (non-negotiable)

1. **Worktree-first.** Never edit outside `noctalia-appmenu-74-bridge-completion/`.
2. **Branch off `origin/main`.** Not `004-project-completion`.
3. **DCO sign-off.** `git commit -s -m "feat(bridge): ..."`. Conventional commits enforced by lefthook.
4. **No push to `main`.** Push only to `74-bridge-completion`.
5. **No PR creation.** Parent opens the PR after reviewing your work.
6. **`cargo test` green before committing each task.** Use the existing fixture-replay harness for niri tests; add new harness for atspi.
7. **Constitution check.** Re-verify all 7 principles in your `plan.md`. Principle I (niri-only): your `FocusSink` trait is an abstraction *door*, NOT a Hyprland/Sway implementation.
8. **No new dependencies** without justification in `plan.md` §Open questions.
9. **MSRV preserved.** Read `bridge/Cargo.toml` `[package]` `rust-version` — your code compiles on that MSRV.
10. **Schema-version lockstep.** If you bump `active.json` from `v=1` to `v=1.1` (adding the `source` field per contract), document the change in `bridge/src/active.rs` and `specs/005-bridge-completion/spec.md`. The plugin side (Lane B) must update in the same release.

## Allowlist of Bash commands

You can invoke (no other shell forms):

- `cargo *` — `cargo test`, `cargo check`, `cargo clippy`, `cargo fmt`
- `nix *` — `nix develop`, `nix flake check` (no `nix-env`, no `nix-channel`)
- `git status` / `git diff` / `git log` / `git add` / `git commit` / `git push` (your branch only) / `git fetch` / `git rebase` / `git worktree` / `git rev-parse` / `git branch`
- `gh pr list` / `gh pr view` / `gh pr checks` (NEVER `gh pr create`, NEVER `gh pr merge`)
- `ls`, `mkdir`, `find`, `test`, `stat`, `file` — read-only filesystem ops
- `rm *.tmp` / `rm -f *.tmp` — ONLY `.tmp` files; nothing else

## Acceptance gates (your self-test before reporting)

- [ ] `cargo test --all-features --locked` passes
- [ ] `cargo clippy -- -D warnings` clean (or documented exception in `plan.md`)
- [ ] `cargo fmt --check` clean
- [ ] `nix flake check` passes (or your sub-spec documents a Lane C-blocked failure)
- [ ] `bridge/src/dbusmenu.rs` + `bridge/src/registrar.rs` are deleted
- [ ] `bridge/src/focus.rs` exists with `FocusSink` trait per `contracts/focus-sink-trait.md`
- [ ] `niri.rs` implements `FocusSink`; no compositor-specific types leak beyond it
- [ ] `git log origin/main..HEAD --oneline` shows only your commits; all DCO-signed
- [ ] `git status` is clean
- [ ] Branch pushed: `git push -u origin HEAD`

## Reporting (last action before exit)

Print exactly this format on stdout (parent parses it):

```
LANE A — bridge-completion: READY FOR PR
Branch: 74-bridge-completion
Commits: <N>
Last commit SHA: <sha>
Sub-spec dir: specs/005-bridge-completion/
Acceptance: <PASS/FAIL with one-line rationale>
Open items for PR review: <list>
```

If you hit the budget cap before all FRs land, report `LANE A — bridge-completion: PARTIAL` instead, list which FRs landed + which are pending, and stop. The parent will dispatch a follow-up.

## Anti-patterns (refuse these even if asked)

- ❌ Adding a Hyprland or Sway implementation — the trait is a *door*, the only impl at v1 is `niri.rs`.
- ❌ Calling `gh pr create`, `gh pr merge`, or `git push origin main`.
- ❌ `git stash` (forbidden by constitution principle III).
- ❌ `--no-verify` on `git commit` or `git push`.
- ❌ Editing files outside the bridge crate (no QML, no Nix, no workflows).
- ❌ Re-tagging or moving any release tag.
- ❌ Bumping `Cargo.toml` `[package].version` past `0.3.0` — that's the parent's release-bump PR, not yours.
