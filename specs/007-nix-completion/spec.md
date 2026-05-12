# Specification: Nix surface completion (v0.3.0 → v1.0.0)

**ID:** 007-nix-completion
**Parent:** 004-project-completion (umbrella)
**Created:** 2026-05-12
**Author:** @phsb5321 (Lane C worker)
**Constitution version:** 1.0.0

## Why

`flake.nix` and `nix/module.nix` shipped v0.3.0 with the AT-SPI substrate (ADR-0024) merged into the bridge but with their Nix surface still describing the pre-AT-SPI DBusMenu world. Concretely: AT-SPI prerequisites (`QT_ACCESSIBILITY=1`, `services.gnome.at-spi2-core.enable`) are not wired; the `registrar` HM option still defaults to `"vala-panel"` and pulls `vala-panel-appmenu` + `appmenu-gtk-module` into `home.packages`; the `noctalia-appmenu-registrar` systemd user unit is still installed; `hideInWindowMenubar` still writes `QT_QPA_PLATFORMTHEME=appmenu-qt5` + `GTK_MODULES=appmenu-gtk-module` which have no effect under AT-SPI; the `bridge` and `plugin` derivations hardcode `version = "0.1.0"` while `bridge/Cargo.toml` is at `"0.3.0"`; `SOURCE_DATE_EPOCH` is derived inside the build sandbox via `git log`, which is non-deterministic and fragile; plugin discovery is documented in spec 001 FR-007 as requiring an entry in `plugins.json` but the HM module only writes the manifest directory.

This sub-spec lands the Nix-surface deliverables of 004 — FR-014 through FR-020 — under one PR. Scope is HM-only (ADR-0011); a NixOS system module mirror stays deferred to v2.

## User scenarios

### Scenario 1: Fresh-NixOS install (spec 004 §Scenario 1 mirror)

**Given** a freshly-installed NixOS host with:
- `services.gnome.at-spi2-core.enable = true`
- `programs.noctalia.plugins.appmenu.enable = true`
- `programs.noctalia-shell.plugins.states.noctalia-appmenu.enabled = true`

**When** the user rebuilds via `nh os switch .` and re-enters the niri+noctalia-shell session
**Then** `QT_ACCESSIBILITY=1` is set in `home.sessionVariables`, the `noctalia-appmenu-bridge` systemd user unit starts under `graphical-session.target`, the plugin manifest is resolvable under `~/.config/noctalia/plugins/noctalia-appmenu/`, and Anki's menubar appears in the noctalia top bar on focus

### Scenario 2: Missing AT-SPI prerequisite warns at eval time

**Given** `programs.noctalia.plugins.appmenu.enable = true` and `services.gnome.at-spi2-core.enable = false` (or unset)
**When** the user runs `home-manager switch` or `nh os switch .`
**Then** evaluation emits an `assertion` failure (NixOS scope) or a `lib.warn` (pure HM scope) telling the user to enable `services.gnome.at-spi2-core` system-wide; the rebuild fails or warns loudly before the user discovers the silent menu-empty state at runtime

### Scenario 3: Migration from v0.3.0 `registrar = "vala-panel"` config

**Given** an existing user upgrading from v0.3.0 final with `programs.noctalia.plugins.appmenu.registrar = "vala-panel"` set explicitly
**When** they pin v1.0.0 in their flake input and rebuild
**Then** the option is still recognised (no eval-error), a `lib.warn` fires telling the user the option is deprecated and will be removed in v1.1, `vala-panel-appmenu` and `appmenu-gtk-module` are no longer pulled into `home.packages`, the `noctalia-appmenu-registrar` systemd user unit is not installed, and the plugin keeps working because the AT-SPI walker does not depend on the registrar

### Scenario 4: Reproducible build asserts version + epoch parity

**Given** the v1.0.0 flake and a release-workflow override `SOURCE_DATE_EPOCH=<commit-timestamp>`
**When** a maintainer runs `nix build .#noctalia-appmenu-bridge` twice on different machines
**Then** both invocations produce byte-identical binaries; the derivation's `version` attribute equals `bridge/Cargo.toml`'s `package.version`; a `nix flake check` step asserts that parity and fails the build when the two diverge

## Functional requirements

### FR-014 — `QT_ACCESSIBILITY=1` unconditional

When `programs.noctalia.plugins.appmenu.enable = true`, the HM module writes `home.sessionVariables.QT_ACCESSIBILITY = "1"`. The write is unconditional w.r.t. `hideInWindowMenubar` and the deprecated `registrar` option. Rationale: Qt apps only register their accessibility tree with the a11y bus when this env var is set; without it, the AT-SPI walker sees an empty tree.

**Acceptance:** module eval with `enable = true` produces `home.sessionVariables.QT_ACCESSIBILITY = "1"`; module eval with `enable = false` does not write the variable.

### FR-015 — AT-SPI system-wide assertion / warning

The module emits an `assertion` (when the surrounding NixOS scope makes them available) or a `lib.warn` (pure HM-on-non-NixOS) telling the user that `services.gnome.at-spi2-core.enable = true` is a prerequisite. The README's "Verify the install" recipe (Lane D / FR-028) lists this as the first check.

**Acceptance:** module eval with `enable = true` and `services.gnome.at-spi2-core.enable = false` triggers the assertion/warning; module eval with both true is clean.

### FR-016 — Deprecate `registrar` option + retire DBusMenu deps

- `registrar` option default flips from `"vala-panel"` to `"none"`.
- Option description gains a DEPRECATED prefix and a v1.1 removal timeline.
- A `lib.warn` fires when the option is set to a non-default value (`"vala-panel"`).
- `home.packages` no longer includes `pkgs.vala-panel-appmenu` or `pkgs.appmenu-gtk-module` under any setting.
- `systemd.user.services.noctalia-appmenu-registrar` is no longer installed (the `lib.mkIf` branch is removed; the unit is dead code post-ADR-0024).

**Acceptance:** module eval with default options does not pull `vala-panel-appmenu` or `appmenu-gtk-module` into `home.packages`, does not install the registrar unit, and does not warn. Setting `registrar = "vala-panel"` warns but still evaluates successfully.

### FR-017 — Remove stale `QT_QPA_PLATFORMTHEME` + `GTK_MODULES` writes

The `home.sessionVariables` block guarded by `hideInWindowMenubar` is rewritten. Either:

- (a) `hideInWindowMenubar` remains as an option but writes nothing (the AT-SPI substrate makes Qt and GTK menubars in-window irrelevant — the bar surface is the only menu surface), with an updated description, OR
- (b) `hideInWindowMenubar` is removed entirely.

This spec picks **(a)** to preserve config compatibility for existing users (the option may still be set; it simply has no effect).

**Acceptance:** module eval with `hideInWindowMenubar = true` does not write `QT_QPA_PLATFORMTHEME` or `GTK_MODULES` in `home.sessionVariables`. Setting the option does not warn (it is forward-compatible, not deprecated).

### FR-018 — Version source-of-truth from `bridge/Cargo.toml`

`bridge` and `plugin` derivations in `flake.nix` derive their `version` attribute from `bridge/Cargo.toml` via `lib.importTOML`. A shared `nix/version.nix` is optional; this spec inlines the read at the call site (single source-of-truth, no extra file). A `nix flake check` derivation asserts version parity (the `version` attribute equals the Cargo manifest's `package.version`); divergence fails the build.

**Acceptance:** `nix eval .#noctalia-appmenu-bridge.version` returns `"0.3.0"` (matching current `Cargo.toml`); editing `Cargo.toml` to `"0.3.1"` makes both `bridge.version` and `plugin.version` re-evaluate to `"0.3.1"` without touching `flake.nix`.

### FR-019 — `SOURCE_DATE_EPOCH` from outside the sandbox

`SOURCE_DATE_EPOCH` is set as a derivation attribute computed from `self.lastModified` (the flake input's last-modified timestamp, exposed by flake-parts) rather than derived inside the sandbox by shelling out to `git log`. The release workflow may override via `SOURCE_DATE_EPOCH` env var consumed by the action before `nix build`. The hardcoded `1735689600` fallback is removed entirely.

**Acceptance:** the `bridge` derivation has `SOURCE_DATE_EPOCH = toString self.lastModified` set as an env attribute; `preBuild` no longer shells out to `git`; building twice with the same flake.lock produces byte-identical binaries.

### FR-020 — Plugin discovery

Investigation findings (recorded here per the brief's "verify upstream behaviour first" instruction):

1. noctalia-shell's `PluginService` loads only plugins listed with `enabled = true` in `~/.config/noctalia/plugins.json::states.<plugin-id>`. Reference: `~/NixOS/modules/home/linux/noctalia/shell.nix` line 211 comment authoritatively states "PluginService only loads plugins listed as `enabled = true` in plugins.json::states. Without this entry, the manifest is discovered + the bar widget id is referenced in bar.widgets.left, but PluginService skips loading."
2. The plugin manifest directory (already wired via `xdg.configFile."noctalia/plugins/noctalia-appmenu"`) is necessary but not sufficient.
3. Writing `~/.config/noctalia/plugins.json` directly from our HM module would conflict with the upstream `programs.noctalia-shell.plugins` option that noctalia-shell's own HM module owns (single-writer rule).

**Decision:** keep the plugin manifest directory write. Document in the option description that users must additionally enable the plugin in noctalia-shell's plugin index, either:
- via `programs.noctalia-shell.plugins.states.noctalia-appmenu.enabled = true` (when the user consumes noctalia-shell's own HM module), or
- via a manual entry in `~/.config/noctalia/plugins.json::states.noctalia-appmenu.enabled = true` (when the user wires noctalia-shell some other way).

**Acceptance:** module eval with `enable = true` writes the manifest directory to `~/.config/noctalia/plugins/noctalia-appmenu/`; the option description text contains the explicit `plugins.json::states` step and references the upstream HM option as the preferred path; the README's "Verify the install" recipe (Lane D) lists this as a documented prerequisite.

## Non-functional requirements

- **NFR-001 Reproducibility.** `nix build .#noctalia-appmenu-bridge` is byte-identical across two invocations sharing one `SOURCE_DATE_EPOCH` value (FR-019).
- **NFR-002 Forward compatibility.** Removing the `registrar` option in v1.1 must not require touching this module's call sites; consumers should be able to upgrade by dropping the line.
- **NFR-003 Pure-eval safe.** No `builtins.getEnv` calls in module code; no `~` in Nix paths; all conditional branches use `lib.mkIf` / `lib.optionals` / `lib.optionalAttrs`.
- **NFR-004 alejandra clean.** All edited files pass `alejandra --check`.

## Out of scope

- NixOS system module mirror (ADR-0011 + plan.md defer to v2).
- Direct write to `~/.config/noctalia/plugins.json` (single-writer conflict — see FR-020).
- Removing the `registrar` option entirely (deferred to v1.1 per FR-016 migration path).
- Reproducibility CI job wiring (Lane D / FR-019 spec coverage; this spec only sets the derivation attribute).
- README "Verify the install" recipe (Lane D / FR-028).

## Constraints / dependencies

- `flake-parts` exposes `self.lastModified` at the flake level; `perSystem` accesses it via `inputs.self.lastModified`.
- `pkgs.lib.importTOML` reads `bridge/Cargo.toml` at evaluation time; no IFD risk because the file is in-tree.
- `crane.buildPackage` honours `SOURCE_DATE_EPOCH` set as a derivation attribute (no `preBuild` override needed).
- `home-manager` exposes `assertions` only on NixOS-host activation; pure HM does not enforce them. `lib.warn` is the cross-context fallback.
- `noctalia-shell` HM module's `programs.noctalia-shell.plugins.states.<id>.enabled = true` is the canonical user-facing wiring step.

## Success criteria

- **SC-001** `nix flake check` clean on the v1.0.0 branch after this spec lands.
- **SC-002** `alejandra --check nix/ flake.nix` clean.
- **SC-003** Module evaluation under all four scenarios (cartesian of `enable × at-spi2-core.enable`) produces the expected assertion/warning per FR-015.
- **SC-004** `nix build .#noctalia-appmenu-bridge` succeeds; `nix eval .#packages.x86_64-linux.noctalia-appmenu-bridge.version` returns the same string as `bridge/Cargo.toml`'s `package.version`.
- **SC-005** Module evaluation with default options does not include `vala-panel-appmenu` or `appmenu-gtk-module` in `home.packages`, does not install `noctalia-appmenu-registrar.service`, and does not write `QT_QPA_PLATFORMTHEME` or `GTK_MODULES` to `home.sessionVariables` under any `hideInWindowMenubar` setting.

## Key entities

- `flake.nix` — derivation definitions for `bridge` and `plugin`; version source-of-truth wiring; `SOURCE_DATE_EPOCH` attribute set from `self.lastModified`.
- `nix/module.nix` — HM module surface; option declarations + config wiring.
- `bridge/Cargo.toml` — version source-of-truth (`package.version`).

## Assumptions

- `self.lastModified` is monotonic-enough for reproducibility purposes (flake lock changes only on input bumps).
- The upstream noctalia-shell HM module remains the canonical wiring path for `plugins.json::states`; if upstream changes the surface, this spec's option description references stay accurate via documentation update, not eval-time logic.
- `lib.importTOML` evaluation cost is negligible (one-time read of a small file).
