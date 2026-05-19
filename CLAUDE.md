# CLAUDE.md — noctalia-appmenu

Project-scoped instructions for AI agents (Claude Code, Cursor, Copilot Workspace) working on this repo. Layered ON TOP of the user's global `~/.claude/CLAUDE.md` and the NixOS repo's `CLAUDE.md` — both still apply. Where they conflict, see the resolution rule at the bottom of `.specify/memory/constitution.md`.

## What this project is

A noctalia plugin that puts a focused app's menubar in the topbar — macOS-style — on niri/Wayland. The hard architectural facts:

1. The plugin alone cannot do this. Quickshell's `DBusMenuHandle` is `QML_UNCREATABLE`. `com.canonical.AppMenu.Registrar`'s `windowId` is an X11 XID, useless on Wayland.
2. So we ship a **Rust sidecar bridge** that follows niri-IPC focus, resolves registrar entries by D-Bus connection PID, and re-exports the active app's menu at a fixed proxy address (`org.noctalia.AppMenu /org/noctalia/AppMenu/Active`).
3. The plugin is then a thin QML widget that subscribes to that fixed proxy.

If you do not understand the constraint above, **read `docs/adr/` from 0001 to 0012 before changing anything in `bridge/src/` or `plugin/`**. The ADRs are short.

## Where to read first

- `.specify/memory/constitution.md` — load-bearing rules. PRs that violate constitution principles are rejected.
- `docs/adr/` — every architectural decision, with the *reason* (the parts not derivable from the code).
- `specs/001-global-menu/` — current MVP spec / plan / tasks.
- `~/NixOS/meta/yolo-labz-release-engineering-research.md` — supply-chain rules this repo inherits (action pinning, SBOM formats, attestation flow). Sections §0 and §3 (Rust) are mandatory reading before touching `.github/workflows/`.

## Hard bans (no exceptions, no asks)

1. **No `git push origin main`.** Feature branches + PR. Repository Ruleset blocks pushes anyway, but don't even try.
2. **No `git stash`.** Open another worktree.
3. **No `--no-verify`.** If a hook fails, fix the underlying issue.
4. **No re-tagging a release.** Cut `vX.Y.Z+1` on botched publishes.
5. **No hand-edits to `CHANGELOG.md`.** `git-cliff` owns it.
6. **No tag-pin on a GitHub Action.** Always full 40-char SHA + `# vX.Y.Z` trailing comment. The comment is what Dependabot's regex needs to recognise the entry — never strip it.
7. **No `USER_TOKEN` in CI for SonarQube.** Always `PROJECT_ANALYSIS_TOKEN` scoped to `yolo-labz_noctalia-appmenu`.
8. **No bus-name acquisition from QML.** That is what the bridge is for.
9. **No introducing a second compositor's focus tracker before niri's is shipping in production.**
10. **No `~` in Nix paths.** Use `config.home.homeDirectory` or `XDG_CONFIG_HOME` resolution.

## Git workflow (the canonical recipe)

```bash
# 1. Setup — main worktree only
cd ~/Documents/Code/noctalia-appmenu
git status              # MUST be clean
git fetch origin main && git pull --ff-only origin main

# 2. Worktree
NN=$(gh pr list --state all --limit 1 --json number -q '.[0].number + 1')
SLUG=fix-submenu-popup
git worktree add ../noctalia-appmenu-${NN}-${SLUG} -b ${NN}-${SLUG} origin/main
cd ../noctalia-appmenu-${NN}-${SLUG}
git log origin/main..HEAD --oneline   # MUST be empty

# 3. Edit, test, commit, rebase, push, PR
# … work in this worktree, not in ~/Documents/Code/noctalia-appmenu …
git add -- <specific files>           # never `git add -A`
git commit -s -m "type(scope): description"
git fetch origin main && git rebase origin/main
git log origin/main..HEAD --oneline   # confirm ONLY your commits
git push -u origin HEAD
gh pr create --title "type(scope): description"
gh pr checks <PR> --watch
gh pr merge <PR> --squash --delete-branch

# 4. Teardown
cd ~/Documents/Code/noctalia-appmenu
git worktree remove ../noctalia-appmenu-${NN}-${SLUG}
git fetch origin main && git pull --ff-only origin main
```

## Releases — invoke the canonical skill

End-to-end release + deploy is codified in **`.claude/commands/release-deploy.md`**
and the underlying `scripts/release.sh`. **Do not improvise.** The skill enforces:

- Tag-on-correct-commit — `plugin-tag` stage refuses to tag if `origin/main`
  HEAD subject does not contain the version. Catches the v1.0.14 drift
  (CLAUDE.md trigger E) mechanically.
- Pre-push tag verification — `scripts/verify-tag-subject.sh` runs in lefthook's
  `pre-push` hook, refuses any local-only `v*` tag whose target commit
  subject lacks the version. Second layer of defence for the same drift.
- NixOS flake bump via PR + admin-merge — never `git push origin main`.
- QML bytecode cache nuke (`rm -rf ~/.cache/noctalia-qs/qmlcache/`)
  immediately before the shell restart. Nix store mtimes are the epoch
  (1969-12-31); Quickshell's mtime-based `.qmlc` freshness check loses
  to the wall-clock-dated cache from the previous release, so the
  restart re-loads yesterday's compiled QML. Symptom: Pedro reports
  "absolutely nothing changes !!" three releases running. Mechanical
  fix — the cache must die or the deploy is invisible.
- `systemctl --user restart noctalia-shell.service` after `nixos-rebuild`
  — closes the "deploy claimed, binary unchanged" gap
  (`feedback_nh_switch_no_shell_restart.md`).
- Final `noctalia-appmenu-bridge --version` check — closes drift trigger G.

Usage:

```bash
# After opening a worktree-first bump PR with VERSION in the title:
scripts/release.sh 1.0.18

# Resume mid-flow after a stage failure:
scripts/release.sh 1.0.18 --skip-stage plugin-merge --skip-stage plugin-tag
```

If you find yourself wanting to run any release step by hand — **stop**.
Add the missing capability to `scripts/release.sh` (single source of truth)
and re-invoke. Hand-rolled release flows are the drift mechanism this skill
exists to suppress.

## Specialised agents (`.claude/agents/`)

| Agent | When to invoke |
|---|---|
| `qml-architect` | New QML widget, refactor of `BarWidget.qml`, popup behaviour change. |
| `dbusmenu-protocol-expert` | Anything touching `com.canonical.dbusmenu`, `AppMenu.Registrar`, or the bridge's re-export proxy. |
| `niri-wayland-tester` | Focus-tracking bugs, niri-IPC schema drift, integration test failures. |
| `nix-packager` | `flake.nix`, `nix/module.nix`, derivation changes. |
| `ci-engineer` | `.github/workflows/`, action SHA pinning, runner labels. |
| `sonar-quality-gate` | SonarQube quality gate, sonar-project.properties, qmllint SARIF upload. |

Invocation pattern (parallel where independent):

```
Agent(qml-architect, "draft submenu popup component")
Agent(dbusmenu-protocol-expert, "validate the proxy interface signature")
Agent(niri-wayland-tester, "design the integration test for focus debouncing")
```

Do not duplicate sub-agent work in the parent context.

## Reference URLs (canonical)

| Topic | URL |
|---|---|
| Quickshell DBusMenu | https://quickshell.org/docs/v0.3.0/types/Quickshell.DBusMenu/ |
| Quickshell Toplevel | https://quickshell.org/docs/v0.3.0/types/Quickshell.Wayland/ToplevelManager |
| niri IPC | https://yalter.github.io/niri/IPC.html |
| AppMenu.Registrar XML | https://github.com/KDE/plasma-workspace/blob/master/appmenu/com.canonical.AppMenu.Registrar.xml |
| dbusmenu spec | https://github.com/AyatanaIndicators/libdbusmenu/blob/master/libdbusmenu-glib/dbus-menu.xml |
| zbus | https://docs.rs/zbus/latest/zbus/ |
| niri-ipc crate | https://docs.rs/niri-ipc/ |

## Infrastructure cheatsheet

| Service | Where | Notes |
|---|---|---|
| GitHub repo | `yolo-labz/noctalia-appmenu` | Public. Apache-2.0. Repository Rulesets active on `main`. |
| CI runner | `vm103.home302server` (192.168.1.113) | Labels: `self-hosted, Linux, X64, vm103, noctalia-appmenu`. SSH host: `runner` / `ProxMox.Runner`. |
| SonarQube | `https://sonarqube.home301server.com.br` | Project key: `yolo-labz_noctalia-appmenu`. Tailscale fallback: `100.99.218.39:9000`. |
| Sonar token | `SONAR_TOKEN` GH secret | `PROJECT_ANALYSIS_TOKEN` scope only. |
| Bridge user unit | `noctalia-appmenu-bridge.service` | `WantedBy=graphical-session.target`. Hardened (`NoNewPrivileges`, `ProtectSystem=strict`). |

## Debugging recipes

### Bridge not publishing the active proxy

```bash
busctl --user list | grep org.noctalia.AppMenu
journalctl --user -u noctalia-appmenu-bridge.service -n 50 --no-pager
RUST_LOG=noctalia_appmenu_bridge=debug noctalia-appmenu-bridge --foreground
```

### Plugin loaded but not rendering

```bash
qs -c noctalia-shell ipc reload          # noctalia/quickshell IPC reload
gdbus introspect --session --dest org.noctalia.AppMenu --object-path /org/noctalia/AppMenu/Active
```

### Focus changes not detected

```bash
niri msg --json event-stream | jq 'select(.type=="WindowFocusChanged")'
niri msg --json windows | jq '.[] | {id, app_id, pid, is_focused}'
```

## Initiative rule

Read-mostly operations (running tests, querying state, reading code, scanning logs): **do them, don't ask**. Account-committing operations (creating GH repos, registering runners, generating Sonar tokens, force-pushing): one confirmation pass with the human, then go.

## Drift detection — MANDATORY pre-action checks

**Full doctrine + case study: [`specs/013-sota-overhaul/agent-governance.md`](specs/013-sota-overhaul/agent-governance.md).** Read it before iterating on any non-trivial bug in this repo.

### Observable drift triggers

Check these before every commit. If ANY fires, the decision tree below is mandatory — `git commit` is BLOCKED until the trigger's branch resolves. The case study (v1.0.5..v1.0.12 = 8 plugin releases on ONE outside-click bug) is the worked failure mode.

| ID | Trigger | Mechanical detection |
|---|---|---|
| **A** | Commit cites a prior version by tag/SHA as the failure mode | `git log -1 --format=%B \| grep -qE "v[0-9]+\.[0-9]+\.[0-9]+'s? (failure\|bug\|race\|regression)"` |
| **B** | More than 3 releases in 24h on one symptom | `git tag --sort=-creatordate \| head -4 \| xargs -I{} git log -1 --format=%ci {}` — span < 24h |
| **C** | Two consecutive ship attempts failed the same smoke | Pedro reports the *same* symptom in successive iterations |
| **D** | Pedro repeats the bug report verbatim | Transcript contains two `<user>` turns with the same noun phrase |
| **E** | Iteration reverts to an earlier architecture with no ADR | `git log --oneline plugin/` grep for "(was: <pattern>)" or "revert" |
| **F** | Static check (qmllint / clippy) green, runtime red | Last release shipped with `qmllint` clean but `journalctl --user -u noctalia-shell` shows QML errors on load |
| **G** | Deploy claimed, running binary unchanged | `noctalia-appmenu-bridge --version` ≠ the tag you just pushed |
| **H** | Commit body says "should" / "expected to" without "verified by" | `git commit -m` containing "should fix" / "expected to" / "will probably" |

### Decision tree (trigger → action → entry command)

| Trigger | Required action | Entry command |
|---|---|---|
| A | Rename failure mode to the ADR/spec-FR ID before next commit | `grep -rn "<symptom phrase>" docs/adr/ specs/*/spec.md` |
| B | STOP tagging. Open a redesign spec instead | `git worktree add ../noctalia-appmenu-NNN-redesign-<bug> -b NNN-redesign origin/main && cp .specify/templates/spec-template.md specs/NNN-<bug>/spec.md` |
| C | Codex adversarial review reading framework source — NOT another agent iteration | Invoke `codex:codex-rescue` agent with prompt: *"be brutal: read /nix/store/\*-quickshell-source/src/ — what still fails in <patch>?"* |
| D | Stop coding. Re-read the original report and ADR chain cold | `git log --grep "<phrase>"`; re-read `docs/adr/` 0001..latest before next edit |
| E | Force-redesign spec. Document why the detour failed before the next forward step | Same as B; the new spec MUST cite every iteration it discards |
| F | Add runtime smoke to CI BEFORE next release | `qml --offscreen -I plugin/ plugin/BarWidget.qml` in `.github/workflows/ci.yml` |
| G | Verify binary state at runtime before iterating | `systemctl --user restart noctalia-shell.service && noctalia-appmenu-bridge --version` |
| H | Block commit; require explicit "verified by ..." smoke evidence in body | Pre-commit hook regex `\b(should\|expected to\|will probably)\b` → fail |

### Alignment guardrails (always-on rules)

1. **Cite failure modes by ADR / spec-FR ID, not by symptom or prior version.** A bug is named "ADR-0024 / FR-002 regression", not "the v1.0.10 problem".
2. **Read upstream framework source before reverting a fix.** Quickshell/niri/zbus sources live under `/nix/store/*-{quickshell,niri,zbus}-*/src/`. Codex caught `deleteOnInvisible()==true` here in the v1.0.4 cycle — that pattern is required, not optional.
3. **Run runtime smoke (not just lint) before claiming a fix.** `qmllint` is NOT a load test; clippy is NOT an integration test. Spec a `[ ]` checkbox item only flips to `[x]` when a passing test reference (journalctl line / CI run URL) is in the same commit.
4. **Verify the running binary loaded the new build before iterating.** `nh os switch` does NOT restart `noctalia-shell.service`. Always `systemctl --user restart noctalia-shell.service && noctalia-appmenu-bridge --version` after deploy.
5. **Isolate one axis per commit/PR.** Aesthetic + engineering changes do not mix — they cannot be partially reverted.
6. **Swarm for *gathering*, collapse to one parent for *deciding*.** Spawn ≥ 3 parallel research agents to gather context; never spawn N parallel agents to "vote" on a decision — judgement does not parallelise.
7. **Two failed iterations on the same bug = the architecture is wrong, not the patch.** Treat C as a hard stop. Iteration 3 of the same architecture is forbidden.

### Anti-patterns (hard forbidden)

- ❌ **Tagging a release with the bug still observable to the user.** Block via pre-tag smoke against the user-reported reproduction.
- ❌ **Writing "this should fix it" without smoke evidence in the same commit.** Pre-commit regex blocks the verb.
- ❌ **Iterating after 2 failures without codex review OR a framework-reading subagent.** Trigger C is non-negotiable.
- ❌ **Reverting to an earlier architecture with NO ADR documenting why the detour failed.** The detour's failure modes are the next agent's only protection against repeating them. v1.0.12 reverted to v1.0.3 — the new ADR (TBD) is owed.
- ❌ **Trusting `nh os switch` exit code as proof a user-space service loaded new code.** Always verify via `--version` or journal.
- ❌ **Mixing aesthetic + engineering changes in one commit/PR.** Per `align-methodology` §6.
- ❌ **Treating vision-agent or lint-agent severity grades as authoritative without Pedro-verification.** P0 calls from AI graders can be wrong.

### Case study — what NOT to do (v1.0.5..v1.0.12)

8 plugin releases in 28 h for ONE bug (outside-click dismiss):

```
v1.0.5  drop recursive Component               (F, G — 4 prior tags never loaded)
v1.0.6  skip-list + 30s cache                  (scope-shift, masks dismiss bug from B)
v1.0.7  restore Firefox + Chromium menus       (A — cites prior)
v1.0.8  parallel walk                          (A)
v1.0.9  outside-click dismisses popup          (D — Pedro re-reported)
v1.0.10 popup→Overlay + permanent shield       (A, C — should have triggered codex)
v1.0.11 shield input via mask Region           (A, C — should have triggered redesign)
v1.0.12 xdg_popup grab                         (A, C, E — reverts to v1.0.3 with no ADR)
```

Where the tree would have stopped the loop:
- After **v1.0.10** (trigger C: 2 failed dismiss iterations), table demands a codex review reading Quickshell's `PanelWindow::popupGrab` source. Not done. Cost: 2 redundant tags (~5h iteration).
- After **v1.0.12** (trigger E: architectural backtrack), table demands a redesign spec documenting why v1.0.9..v1.0.11 failed. **THIS spec (`specs/013-sota-overhaul/`) is that owed document.**

Lesson: the defence against drift is not smarter agents, it is **mechanical triggers** on observable git/transcript signals. Wire A–H into pre-commit hooks and release checklists; agent judgement fails under iteration pressure, a regex does not.

## What is *not* in this file

Cross-project rules (date format, Brazilian DD/MM/YYYY, commit-style, hardware saturation) live in `~/.claude/CLAUDE.md` — read it. Yolo-labz release-engineering rules (SBOM formats, action pin policy) live in `~/NixOS/meta/yolo-labz-release-engineering-research.md` — read it before any workflow changes.
