# Requirements checklist — Nix surface completion (Lane C)

**Spec:** `specs/007-nix-completion/spec.md`

Per-FR acceptance checks. Each item is binary: ✓ on green, ✗ on red.

## FR-014 — `QT_ACCESSIBILITY=1` unconditional

- [ ] Module eval with `cfg.enable = true` writes `home.sessionVariables.QT_ACCESSIBILITY = "1"`.
- [ ] Module eval with `cfg.enable = false` does not write the variable.
- [ ] Write does not depend on `cfg.hideInWindowMenubar` or `cfg.registrar`.

## FR-015 — AT-SPI system-wide assertion / warning

- [ ] `assertion` entry present in `config.assertions` when the surrounding NixOS scope provides it.
- [ ] `lib.warn` fires at module-eval time when `cfg.enable = true` AND `config.services.gnome.at-spi2-core.enable = false`.
- [ ] Eval is clean when both `cfg.enable = true` AND `at-spi2-core.enable = true`.

## FR-016 — Deprecate `registrar` + retire DBusMenu deps

- [ ] `registrar` option default is now `"none"`.
- [ ] Option description starts with `DEPRECATED.` and documents the v1.1 removal timeline.
- [ ] `lib.warn` fires when `cfg.registrar != "none"`.
- [ ] `home.packages` does not include `pkgs.vala-panel-appmenu` under any setting.
- [ ] `home.packages` does not include `pkgs.appmenu-gtk-module` under any setting.
- [ ] `systemd.user.services.noctalia-appmenu-registrar` is no longer declared (no `lib.mkIf` branch, no Service block).

## FR-017 — Remove stale env-var writes

- [ ] `home.sessionVariables.QT_QPA_PLATFORMTHEME` is not written under any setting (grep clean).
- [ ] `home.sessionVariables.GTK_MODULES` is not written under any setting (grep clean).
- [ ] `hideInWindowMenubar` option still exists for config compatibility; description notes the no-op behaviour under AT-SPI.

## FR-018 — Version source-of-truth

- [ ] `flake.nix` reads `bridge/Cargo.toml` via `lib.importTOML`.
- [ ] Both `bridge` and `plugin` derivations use the imported version.
- [ ] No hardcoded `"0.1.0"` (or any other literal version) remains in `flake.nix`.
- [ ] `nix eval .#packages.x86_64-linux.noctalia-appmenu-bridge.version` matches `bridge/Cargo.toml`'s `package.version`.

## FR-019 — `SOURCE_DATE_EPOCH` injection

- [ ] `bridge` derivation sets `SOURCE_DATE_EPOCH = toString inputs.self.lastModified;` as a derivation attribute.
- [ ] The `preBuild` block calling `git log` is removed.
- [ ] No hardcoded `1735689600` fallback remains anywhere.
- [ ] `builtins.getEnv` is not used.

## FR-020 — Plugin discovery

- [ ] Module installs the manifest directory via `xdg.configFile."noctalia/plugins/noctalia-appmenu"`.
- [ ] Module does NOT write `~/.config/noctalia/plugins.json` directly.
- [ ] Option description (on `enable` or `registrar`) references `programs.noctalia-shell.plugins.states.noctalia-appmenu.enabled = true` as the canonical user-side enable step.

## Cross-cutting

- [ ] `alejandra --check nix/ flake.nix` clean.
- [ ] `nix flake check` clean.
- [ ] `nix build .#noctalia-appmenu-bridge` succeeds.
- [ ] All commits DCO-signed (`git commit -s`).
- [ ] All commits follow Conventional Commits.
- [ ] Branch `76-nix-completion` pushed; no PR created by this worker.

## Anti-pattern audit

- [ ] No NixOS module mirror added.
- [ ] `registrar` option still present (deprecated, not removed).
- [ ] No hardcoded epoch fallback.
- [ ] No `vala-panel-appmenu` or `appmenu-gtk-module` added back.
- [ ] No `builtins.getEnv` in module code.
- [ ] No `~` in Nix paths (uses `config.home.homeDirectory` / `xdg.configFile.*` resolution).
