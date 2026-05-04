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

## What is *not* in this file

Cross-project rules (date format, Brazilian DD/MM/YYYY, commit-style, hardware saturation) live in `~/.claude/CLAUDE.md` — read it. Yolo-labz release-engineering rules (SBOM formats, action pin policy) live in `~/NixOS/meta/yolo-labz-release-engineering-research.md` — read it before any workflow changes.
