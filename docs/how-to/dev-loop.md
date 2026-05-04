# Local dev loop

The fastest path from "edit" to "verified".

## One-time setup

```bash
git clone git@github.com:yolo-labz/noctalia-appmenu.git
cd noctalia-appmenu
direnv allow                 # pulls the Nix devShell on cd-in
just lefthook-install        # writes hooks into .git/hooks/
```

## Edit

Pick a worktree (per [CONTRIBUTING.md](../../CONTRIBUTING.md)):

```bash
NN=$(gh pr list --state all --limit 1 --json number -q '.[0].number + 1')
git worktree add ../noctalia-appmenu-${NN}-feature -b ${NN}-feature origin/main
cd ../noctalia-appmenu-${NN}-feature
```

## Verify

Two-tier feedback loop:

1. **Pre-commit (~3 s)** — automatic on `git commit -s`. Runs glob-filtered checks on staged files only: `cargo fmt`, `qmllint`, `alejandra`, `deadnix`, `statix`, `gitleaks`, `typos`, `actionlint`.
2. **Pre-push (~30-90 s warm)** — automatic on `git push`. Runs the **full** CI suite in parallel locally — see [ADR-0014](../adr/ADR-0014-local-first-ci.md).

On-demand:

- `just shadow-ci` — run pre-push pipeline without pushing.
- `just fix` — auto-fix everything fixable (`cargo fmt`, `alejandra`, `qmlformat`, `typos --write-changes`).
- `just bridge-test` — quick `cargo test` only.
- `just plugin-lint` — quick `qmllint` only.
- `just nix-flake-check` — quick `nix flake check`.

## Run

Foreground bridge for manual smoke:

```bash
just bridge-run-fg
```

Plugin live-reload (after editing QML):

```bash
just plugin-install-local
qs -c noctalia-shell ipc reload
```

## Debug

```bash
busctl --user list | grep AppMenu
gdbus introspect --session --dest org.noctalia.AppMenu --object-path /org/noctalia/AppMenu/Active
journalctl --user -u noctalia-appmenu-bridge.service -f
niri msg --json event-stream | jq 'select(.type=="WindowFocusChanged")'
```

## Push

```bash
git push -u origin HEAD     # lefthook pre-push runs the full suite
gh pr create --title "..."  # standard PR flow
```

`gh pr checks --watch` to follow remote CI; mostly a verification step at this point.
