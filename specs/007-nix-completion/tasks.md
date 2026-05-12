# Tasks: Nix surface completion (Lane C)

**Spec:** `specs/007-nix-completion/spec.md`
**Plan:** `specs/007-nix-completion/plan.md`

Dependency-ordered, each task self-contained, each commit DCO-signed.

## T-001 — Write sub-spec chain

**Files:**
- `specs/007-nix-completion/spec.md`
- `specs/007-nix-completion/plan.md`
- `specs/007-nix-completion/tasks.md` (this file)
- `specs/007-nix-completion/checklists/requirements.md`

**Acceptance:** all four files committed under one DCO-signed `docs(speckit):` commit. No code changes in this commit.

## T-002 — FR-018 + FR-019 wiring in `flake.nix`

**Files:** `flake.nix`

**Edits:**
1. Bind `cargoToml = pkgs.lib.importTOML ./bridge/Cargo.toml;` at the top of `perSystem.let` block (after `craneLib` definition).
2. Bind `version = cargoToml.package.version;` immediately after.
3. Replace `version = "0.1.0";` in both `bridge` and `plugin` derivations with `inherit version;`.
4. In `bridge` (the `craneLib.buildPackage` call): remove the `preBuild` block entirely; add `SOURCE_DATE_EPOCH = toString inputs.self.lastModified;` as a derivation env attribute (top-level attr, not under `nativeBuildInputs`).
5. Verify `craneLib.devShell` is untouched.

**Acceptance:**
- `nix eval .#packages.x86_64-linux.noctalia-appmenu-bridge.version` returns `"0.3.0"`.
- `nix eval .#packages.x86_64-linux.noctalia-appmenu-plugin.version` returns `"0.3.0"`.
- `git grep '"0.1.0"' flake.nix` → empty.
- `git grep 'preBuild' flake.nix` → empty.
- `git grep 'SOURCE_DATE_EPOCH' flake.nix` → one match (the attribute set assignment).

**Commit:** `feat(nix): version from Cargo.toml + SOURCE_DATE_EPOCH from self.lastModified`

## T-003 — FR-014, FR-015, FR-016, FR-017, FR-020 wiring in `nix/module.nix`

**Files:** `nix/module.nix`

**Edits:**

1. **Option tree changes:**
   - `registrar`: flip default from `"vala-panel"` to `"none"`; prefix description with `DEPRECATED. ...`; document the v1.1 removal.
   - `hideInWindowMenubar`: rewrite description; the option is preserved but is a no-op under AT-SPI (its previous env-write effect is removed; see FR-017).

2. **Config block changes:**
   - Add `assertions = [{ assertion = config.services.gnome.at-spi2-core.enable or false; message = ...; }];` (FR-015 NixOS scope).
   - Wrap the module body in a `lib.warnIf` chain at the top level that fires when `cfg.enable && (config.services.gnome.at-spi2-core.enable or false) == false` (FR-015 HM fallback when assertions are unavailable).
   - Add `home.sessionVariables.QT_ACCESSIBILITY = "1";` unconditional inside the `mkIf cfg.enable` block (FR-014).
   - Remove the `lib.mkIf cfg.hideInWindowMenubar { QT_QPA_PLATFORMTHEME ...; GTK_MODULES ...; }` block (FR-017).
   - Remove the `lib.optionals (cfg.registrar == "vala-panel") [pkgs.vala-panel-appmenu pkgs.appmenu-gtk-module]` branch from `home.packages` (FR-016).
   - Remove the `systemd.user.services.noctalia-appmenu-registrar = lib.mkIf (cfg.registrar == "vala-panel") {...}` block entirely (FR-016).
   - Wrap module return in a `lib.warnIf (cfg.enable && cfg.registrar != "none") "..."` (FR-016).
   - Keep `xdg.configFile."noctalia/plugins/noctalia-appmenu"` write (FR-020 — directory install).
   - Extend the `registrar` option description with the explicit user-side enable instruction (`programs.noctalia-shell.plugins.states.noctalia-appmenu.enabled = true`).

3. **Formatting:** alejandra-clean before commit.

**Acceptance:**
- `nix eval --impure --expr` smoke tests below all return the expected values:
  - `enable=false`: `QT_ACCESSIBILITY` absent from session variables.
  - `enable=true, at-spi2-core.enable=true, registrar="none"` (default): clean module eval, `QT_ACCESSIBILITY="1"` present, no warnings, no `vala-panel-appmenu` in `home.packages`.
  - `enable=true, at-spi2-core.enable=false, registrar="none"`: assertion fires (NixOS) / warning (HM).
  - `enable=true, at-spi2-core.enable=true, registrar="vala-panel"`: warning about deprecation.
- `git grep 'QT_QPA_PLATFORMTHEME' nix/module.nix` → empty.
- `git grep 'GTK_MODULES' nix/module.nix` → empty.
- `git grep 'vala-panel-appmenu' nix/module.nix` → empty (the option description still contains the word `vala-panel` per FR-016; that is intentional).
- `git grep 'noctalia-appmenu-registrar' nix/module.nix` → empty.

**Commit:** `refactor(nix): AT-SPI prerequisites + deprecate registrar option (FR-014…FR-017, FR-020)`

## T-004 — Format + lint pass

**Files:** any files modified above.

**Edits:** `alejandra` over `nix/` and `flake.nix`.

**Acceptance:** `alejandra --check nix/ flake.nix` clean.

**Commit:** included in T-002 / T-003 commits (alejandra runs before each commit). If a follow-up formatting commit is needed, prefix with `style(nix):`.

## T-005 — Verification

**Edits:** none.

**Commands run (allowlisted):**
- `alejandra --check nix/ flake.nix`
- `nix flake check`
- `nix build .#noctalia-appmenu-bridge` (subject to crane fetch budget on first run)
- `nix eval .#packages.x86_64-linux.noctalia-appmenu-bridge.version` — must equal Cargo.toml's `package.version`.
- Cartesian eval smoke (4 scenarios) via `nix-instantiate --eval --expr '<expr>'`.

**Acceptance:** all six commands pass; results captured in the parent reporting block.

**Commit:** verification log only — no file change. If everything is clean, no separate commit is needed; mention results in the parent report instead.

## T-006 — Push branch

**Edits:** none.

**Commands:** `git push -u origin 76-nix-completion`. Branch is `76-nix-completion`; no PR creation per brief.

**Acceptance:** `gh pr list` shows no PR on this branch (worker forbidden from `gh pr create`); branch is pushed and ready for the parent to open the PR.

## Anti-tasks (explicit non-actions)

- Do NOT create a NixOS module mirror under `nix/system-module.nix`.
- Do NOT remove the `registrar` or `hideInWindowMenubar` option entirely (only deprecate / strip effect).
- Do NOT write `~/.config/noctalia/plugins.json` from this module.
- Do NOT add `vala-panel-appmenu` back; do not reintroduce `appmenu-gtk-module`.
- Do NOT call `builtins.getEnv` in module code.
- Do NOT use `~` in Nix paths.
- Do NOT hardcode a `1735689600` fallback for `SOURCE_DATE_EPOCH`.
- Do NOT `gh pr create` — parent owns PR creation.
