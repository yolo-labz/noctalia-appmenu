# Implementation plan: Nix surface completion (Lane C)

**Spec:** `specs/007-nix-completion/spec.md`
**Parent:** 004-project-completion (umbrella)
**Constitution version:** 1.0.0
**Generated:** 2026-05-12

## Approach

This lane has zero file-collision risk with Lanes A, B, D — only `flake.nix` and `nix/module.nix` are touched. Edits are made surgically; the existing systemd hardening block stays intact, the existing `bridge.config` TOML generation stays intact, and the existing devShell stays untouched.

The work splits into three orthogonal slices:

1. **Module rewrite (`nix/module.nix`)** — implements FR-014, FR-015, FR-016, FR-017, FR-020 in one cohesive edit because the option tree, config block, and home-packages list change together.
2. **Flake derivation rewrite (`flake.nix`)** — implements FR-018 (version from `Cargo.toml`) and FR-019 (`SOURCE_DATE_EPOCH` from `self.lastModified`) in one cohesive edit because the `bridge` and `plugin` derivations share the version derivation and the `bridge` derivation's `preBuild` block needs replacing.
3. **Verification** — `alejandra --check`, `nix flake check`, `nix-instantiate --eval` cartesian, `nix build .#noctalia-appmenu-bridge`.

Each slice commits separately with a DCO sign-off; the final commit is the verification slice (no code change but the verification log entry in commit body if anything surfaced).

## Constitution check

| Principle | Status | Notes |
|---|---|---|
| I — niri-only v1 | PASS | No compositor abstraction touched. |
| II — Sidecar by default | PASS | No bus-name acquisition moves into QML. |
| III — Worktree-first git | PASS | Lane C operates exclusively in `../noctalia-appmenu-76-nix-completion/`. |
| IV — Conventional Commits + DCO | PASS | Every commit `-s`-signed with `feat(nix):` / `refactor(nix):` / `chore(nix):` prefix. |
| V — Speckit-driven | PASS | This sub-spec lives under `specs/007-nix-completion/`. |
| VI — Release-engineering | PASS | `SOURCE_DATE_EPOCH` injection is a hardening, not a regression. |
| VII — Graceful degradation | PASS | Deprecation path for `registrar` warns instead of erroring; eval-time assertion is gated on availability (`lib.warn` fallback for non-NixOS HM). |

All gates green.

## Architecture sketch

```
flake.nix
├── inputs: nixpkgs, flake-parts, rust-overlay, crane
└── outputs:
    ├── self (passed down via flake-parts)
    │   └── self.lastModified — fed to bridge derivation's SOURCE_DATE_EPOCH
    └── perSystem
        ├── cargoToml = lib.importTOML ./bridge/Cargo.toml
        ├── version = cargoToml.package.version           # FR-018
        ├── bridge = craneLib.buildPackage {
        │     pname = "noctalia-appmenu-bridge";
        │     version = version;                          # FR-018
        │     SOURCE_DATE_EPOCH = toString self.lastModified;  # FR-019
        │     # preBuild git-log shellout removed
        │   }
        ├── plugin = stdenvNoCC.mkDerivation {
        │     pname = "noctalia-appmenu-plugin";
        │     version = version;                          # FR-018 (shared)
        │   }
        └── checks.version-parity = derivation asserting
              version == cargoToml.package.version       # FR-018 acceptance gate

nix/module.nix
├── options.programs.noctalia.plugins.appmenu
│   ├── enable, package, bridge.{package,niriPackage,config}  (unchanged)
│   ├── registrar = "none" default (was "vala-panel")  # FR-016
│   ├── hideInWindowMenubar (semantics rewrite)        # FR-017
│   └── widgetPlacement (unchanged)
└── config (mkIf cfg.enable):
    ├── assertions = [ { at-spi2-core.enable }]        # FR-015 NixOS
    ├── home.sessionVariables.QT_ACCESSIBILITY = "1"   # FR-014
    │   (no more QT_QPA_PLATFORMTHEME / GTK_MODULES)   # FR-017
    ├── home.packages = [ plugin bridge ]              # FR-016 (no vala-panel deps)
    ├── xdg.configFile."noctalia/plugins/noctalia-appmenu" # FR-020 (unchanged shape)
    ├── xdg.configFile."noctalia-appmenu-bridge/config.toml" (unchanged)
    ├── systemd.user.services.noctalia-appmenu-bridge (unchanged)
    └── (registrar systemd unit removed entirely)      # FR-016

    + lib.warn at the module-top level when registrar != "none"  # FR-016
    + lib.warn at the module-top level when assertions unavailable
      AND at-spi2-core not detected                    # FR-015 HM fallback
```

## Affected files

- `flake.nix` (modified) — FR-018 + FR-019 wiring, `preBuild` git-log shellout removed
- `nix/module.nix` (modified) — FR-014, FR-015, FR-016, FR-017, FR-020 wiring
- `specs/007-nix-completion/spec.md` (new)
- `specs/007-nix-completion/plan.md` (this file, new)
- `specs/007-nix-completion/tasks.md` (new)
- `specs/007-nix-completion/checklists/requirements.md` (new)

No new files under `nix/`; no NixOS module mirror; no `nix/version.nix` (single source-of-truth is `bridge/Cargo.toml`).

## Risks

- **R1** `self.lastModified` is exposed as an attribute on the flake outputs (`inputs.self.lastModified` inside `perSystem`). If a flake-parts upgrade changes the access pattern, the derivation breaks. *Mitigation:* the value is a plain integer; if flake-parts drops the attribute, the workflow fallback (env-var override) still works. Documented in spec FR-019.
- **R2** `lib.importTOML` evaluation might be re-run on every `nix flake check`, adding eval-time cost. *Mitigation:* the file is <2KB and read once per `perSystem`; cost is negligible.
- **R3** Existing users with `registrar = "vala-panel"` see a warning on every rebuild. *Mitigation:* the warning is the migration signal; FR-016 documents that the option is removed in v1.1.
- **R4** `home.packages` change drops `vala-panel-appmenu` from the user's profile. If a user transitively depends on it for something else, their config breaks. *Mitigation:* the package was always installed only as a side-effect of `registrar = "vala-panel"`; users who want it can install it via their own `home.packages` explicitly. Documented in the migration table in `contracts/hm-module-options.md`.
- **R5** `nix flake check` may not currently exercise the cartesian assertion scenarios. *Mitigation:* the check derivation added in FR-018 is the version-parity gate; the FR-015 cartesian eval is covered by `nix-instantiate --eval` smoke checks documented in tasks.md, not by a CI-gated check (Lane D's job).

## Rollout

1. Write spec + plan + tasks + checklist (this commit).
2. Implement FR-018 + FR-019 (`flake.nix`).
3. Implement FR-014 + FR-015 + FR-016 + FR-017 + FR-020 (`nix/module.nix`).
4. `alejandra --check nix/ flake.nix`; if dirty, format + recommit.
5. `nix flake check` — must pass.
6. `nix build .#noctalia-appmenu-bridge` — must succeed; `version` attribute matches `Cargo.toml`.
7. Cartesian eval smoke: 4 scenarios (`enable × at-spi2-core.enable`).
8. Push branch `76-nix-completion`. Report `READY FOR PR` to parent. **No** `gh pr create` from this worker.

## Open questions

1. Should the `noctalia-shell` HM module wiring be referenced directly in the option description (with a code-example block)? *Default*: yes — the description text includes a `programs.noctalia-shell.plugins.states.noctalia-appmenu.enabled = true` example so users coming from a search for "noctalia-appmenu plugin won't load" land on the answer.
2. Should we keep `hideInWindowMenubar` as an option after FR-017 strips its effect? *Default*: yes — preserve the option (with updated description noting it is a no-op under AT-SPI) so existing user configs don't eval-error. Removal in v1.1 alongside `registrar`.
