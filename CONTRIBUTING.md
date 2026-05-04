# Contributing

## Worktree-first workflow (mandatory)

This repo uses the same worktree-first git workflow as `~/NixOS`. Every feature branch lives in its own `git worktree` directory. **Never** `git checkout -b` (or `git switch -c`) inside the main worktree.

```bash
cd ~/Documents/Code/noctalia-appmenu                          # main worktree, on main
git fetch origin main && git pull --ff-only origin main

NN=42 SLUG=fix-submenu-popup
git worktree add ../noctalia-appmenu-${NN}-${SLUG} -b ${NN}-${SLUG} origin/main
cd ../noctalia-appmenu-${NN}-${SLUG}                          # work here

# … edit, test, commit, rebase on origin/main, push, open PR …

cd ~/Documents/Code/noctalia-appmenu
git worktree remove ../noctalia-appmenu-${NN}-${SLUG}
```

Never `git stash`. If you reach for it, you are violating the rule — open a fresh worktree for the interrupting work.

## Conventional commits

Subject under 72 chars. Body explains the *why*, not the *what*. Examples:

- `feat(plugin): render submenu popups via PopupWindow`
- `fix(bridge): debounce focus changes at 75ms`
- `chore(ci): pin sonarsource/sonarqube-scan-action to v7.1.0 sha`

Lefthook + commitlint enforce this on every commit.

## Speckit-first

For non-trivial changes, write a spec first:

```bash
/speckit.specify "feature description"
/speckit.clarify
/speckit.plan
/speckit.tasks
/speckit.implement
```

Specs live under `specs/NNN-slug/`. PR descriptions reference the spec ID.

## DCO sign-off

Every commit must be signed off:

```bash
git commit -s -m "fix(bridge): ..."
```

The CI gate rejects commits without `Signed-off-by:` trailers.

## Testing requirements

- **Bridge changes:** `cargo test` must pass; new code paths covered with `mockall`-faked traits.
- **Plugin changes:** `qmllint` clean; integration test under `nixos-shell` with `niri --headless` if focus / D-Bus behaviour changes.
- **Cross-cutting:** `nix flake check` green.

## Release-engineering invariants

This repo is a yolo-labz plugin. The full standard lives in [`SECURITY.md`](SECURITY.md) and `~/NixOS/meta/yolo-labz-release-engineering-research.md`.

Hard rules:

1. Never re-tag a release. Cut `vX.Y.Z+1` on botched publishes.
2. Never strip the trailing `# vX.Y.Z` comment from a SHA-pinned action.
3. Never edit `CHANGELOG.md` by hand — `git-cliff` owns it.
4. `permissions: {}` workflow-level, re-grant per job.

## Reviewing PRs

Use the bundled `code-review` slash command. The repo's `.claude/agents/` ship with project-tuned reviewers (`qml-architect`, `dbusmenu-protocol-expert`, `niri-wayland-tester`).
