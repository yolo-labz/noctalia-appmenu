# Lane C worker brief — `007-nix-completion`

You are a focused claude-code worker assigned **Lane C** of the `noctalia-appmenu` v1.0.0 roadmap.

## Mission (one paragraph)

Land the Nix surface (`flake.nix` + `nix/module.nix`) for v1.0.0 per the umbrella spec `004-project-completion`. Specifically: wire AT-SPI prerequisites (`QT_ACCESSIBILITY=1` env var + assertion on `services.gnome.at-spi2-core.enable`), deprecate the now-dead `registrar` option + the `vala-panel-appmenu` / `appmenu-gtk-module` package deps + the `noctalia-appmenu-registrar` systemd user unit (ADR-0024 retired DBusMenu), remove stale `QT_QPA_PLATFORMTHEME` / `GTK_MODULES` env writes from `hideInWindowMenubar`, fix the flake version drift (Cargo `0.3.0` vs. flake `0.1.0`), inject `SOURCE_DATE_EPOCH` from outside the sandbox, and resolve plugin discovery (verify whether `plugins.json` needs an explicit entry or directory-scanning suffices). Implement under your own sub-spec at `specs/007-nix-completion/`.

## Source of truth (read in this order, all paths absolute)

1. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/spec.md` — read §User scenarios 1, 5, §Functional requirements §Nix, §Constraints, §SCs
2. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/plan.md` — §Approach + §Affected files §Lane C + §Risks R3
3. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/research.md` — §5 (Nix surface audit)
4. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/contracts/hm-module-options.md` — full HM option tree contract
5. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/docs/adr/ADR-0011-home-manager-module.md`, `ADR-0024-atspi-substrate.md`
6. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/flake.nix`, `nix/module.nix` (current state at v0.3.0 final)
7. `/home/notroot/.claude/CLAUDE.md` (user's global rules — Nix conventions section)

## Your worktree

```bash
cd ~/Documents/Code/yolo-labz/noctalia-appmenu
git fetch origin main
git worktree add ../noctalia-appmenu-76-nix-completion -b 76-nix-completion origin/main
cd ../noctalia-appmenu-76-nix-completion
```

## Your branch

`76-nix-completion` off `origin/main`.

## FRs assigned to you

- **FR-014** `QT_ACCESSIBILITY = "1"` unconditionally when `programs.noctalia.plugins.appmenu.enable = true`
- **FR-015** assertion / warning requiring `services.gnome.at-spi2-core.enable = true`
- **FR-016** deprecate `registrar` option; remove vala-panel-appmenu + appmenu-gtk-module deps; gate `noctalia-appmenu-registrar` unit
- **FR-017** remove or replace `QT_QPA_PLATFORMTHEME` + `GTK_MODULES` writes (stale under AT-SPI substrate)
- **FR-018** `version` source-of-truth — derive from `bridge/Cargo.toml` via `lib.importTOML` or shared `nix/version.nix`
- **FR-019** `SOURCE_DATE_EPOCH` injected from outside the sandbox (`self.lastModified`-derived or release-workflow override)
- **FR-020** plugin discovery — verify noctalia-shell's loader behaviour; if directory-scanning works, no `plugins.json` write; otherwise generate it from the HM module

## Your speckit chain

```
specs/007-nix-completion/{spec.md, plan.md, tasks.md, checklists/requirements.md}
```

## Hard constraints

1. **Worktree-first.** Never edit outside `noctalia-appmenu-76-nix-completion/`.
2. **DCO sign-off + conventional commits.** `git commit -s -m "refactor(nix): ..."` or `feat(nix): ...`.
3. **No push to `main`. No PR creation.**
4. **`nix flake check` clean before committing each task.**
5. **`alejandra` formatting** — 2-space indent, trailing commas, `lib.mkIf` preferred over `if-then-else`, `lib.optionals` / `lib.optionalAttrs` for conditionals.
6. **No `builtins.getEnv` in module code** (fails pure eval).
7. **No `~` in Nix paths.** Use `config.home.homeDirectory` or `XDG_CONFIG_HOME` resolution.
8. **Stay HM-only.** Do NOT create a NixOS module mirror — ADR-0011 + plan.md defer to v2.
9. **Migration path.** `registrar` option stays present in v1.0.0 (deprecated, warns) — actual removal is v1.1.

## Allowlist of Bash commands

- `nix *` — `nix develop`, `nix flake check`, `nix build`, `nix-instantiate --eval`
- `alejandra *` — formatter (run on every changed file before commit)
- `git status` / `git diff` / `git log` / `git add` / `git commit` / `git push` (your branch only) / `git fetch` / `git rebase` / `git worktree` / `git rev-parse` / `git branch`
- `gh pr list` / `gh pr view` / `gh pr checks` (NEVER `gh pr create`, NEVER `gh pr merge`)
- `ls`, `mkdir`, `find`, `test`, `stat`, `file`

## Acceptance gates

- [ ] `nix flake check` clean
- [ ] `alejandra --check nix/ flake.nix` clean
- [ ] Module evaluation under all four scenarios (cartesian of `enable × at-spi2-core.enable`) produces the right assertion/warning
- [ ] `nix build .#noctalia-appmenu-bridge` succeeds; `version` attribute matches `Cargo.toml`
- [ ] Plugin discovery wired (manifest dir + optional plugins.json based on your verification)
- [ ] All commits DCO-signed
- [ ] Branch pushed

## Reporting

```
LANE C — nix-completion: READY FOR PR
Branch: 76-nix-completion
Commits: <N>
Last commit SHA: <sha>
Sub-spec dir: specs/007-nix-completion/
Acceptance: <PASS/FAIL with one-line rationale>
Open items for PR review: <list>
```

## Anti-patterns

- ❌ NixOS module mirror (system-level) — defer to v2 per ADR-0011.
- ❌ Removing the `registrar` option entirely in v1.0.0 — only deprecate.
- ❌ Hardcoded `1735689600` fallback in `SOURCE_DATE_EPOCH` — derive deterministically.
- ❌ Adding `vala-panel-appmenu` back; it is dead post-ADR-0024.
