# ADR-0027 — Drop osConfig from HM module to avoid eval recursion

- **Status:** accepted
- **Date:** 2026-05-12
- **Deciders:** Pedro H S Balbino
- **Supersedes:** none
- **Amends:** spec 004 §FR-015 (assertion mechanism)
- **Tracking PR / branch:** `80-osconfig-fix`

## Context

Lane C (PR #75) added an `osConfig`-based AT-SPI prerequisite probe to `nix/module.nix`:

```nix
self: { config, lib, pkgs, osConfig ? null, ... }: let
  atSpiEnabled =
    if osConfig == null
    then false
    else osConfig.services.gnome.at-spi2-core.enable or false;
in {
  config = lib.warnIf (cfg.enable && !atSpiEnabled) "..." (
    lib.mkIf cfg.enable {
      assertions = lib.optionals (osConfig != null) [...];
      ...
    }
  );
}
```

When the module is consumed via `home-manager.users.<u>.imports` on a NixOS host (the standard composition), `nh os switch` failed with:

```
… while evaluating the option `home-manager.users.notroot._module.freeformType':
… while evaluating the module argument `config' in "/.../modules/home/linux/noctalia:anon-2":
error: infinite recursion encountered
```

Reproduced on Pedro's desktop 12/05/2026 immediately after updating `flake.lock` from `6ad9c2d` (v0.3.0 final) to `a953590` (v1.0.0-rc.2 + deploy-pages bump).

## Cause

The Nix module system evaluates `home-manager.users.<u>._module.freeformType` to know which options are valid in the user's HM scope. That evaluation walks every option declaration, which requires `config` to be resolved. The HM module's top-level `osConfig` binding triggers a read of the surrounding NixOS config — which is what `freeformType` is in the middle of evaluating. Cycle.

`lib.warnIf cond msg val` evaluates `cond` eagerly at module-merge time. With `cond = cfg.enable && !atSpiEnabled` and `atSpiEnabled` reading `osConfig.services.gnome.at-spi2-core.enable`, the cycle fires unconditionally on any HM-in-NixOS composition where `cfg.enable` is true.

`assertions` has the same trap when its content references `osConfig` and is built outside `lib.mkIf`.

## Decision

**Stop reading `osConfig` from the HM module.** Specifically:

- Drop the `osConfig ? null` argument.
- Drop the `atSpiEnabled` binding.
- Move all conditional logic inside `config = lib.mkIf cfg.enable { ... }` so the module system evaluates it lazily on the (already-known) `enable` flag.
- Surface the AT-SPI prerequisite reminder via the `warnings` option (collected at activation time) instead of an eval-time `lib.warnIf`.
- Document the prerequisite in the README's "Verify the install" section (FR-028) and in the option `description`.

## Consequences

### Positive

- `nh os switch .` succeeds on the Pedro composition (NixOS host + HM users imports).
- The module no longer participates in the freeformType evaluation cycle.
- `warnings` are still surfaced at `nh os switch` activation time — the user signal is preserved.
- Pure-HM-on-non-NixOS composition is unchanged (same `warnings`, no NixOS-side hookup expected).

### Negative

- The module no longer asserts (hard-fails) when `services.gnome.at-spi2-core.enable = false`. It only warns. A user who ignores the warning will see an empty bar menu instead of an eval error. Trade-off accepted: eval-time hard failure was producing infinite recursion, runtime soft failure is the safer fallback. README + manifest description carry the prerequisite text.
- ADR-0024 §Failure-modes #1 (`org.a11y.Status.IsEnabled = false`) gains additional weight: the bridge's runtime fallback (synthetic `.desktop` menu) is now the only failure boundary on a misconfigured host.
- FR-015 assertion is downgraded to FR-015 warning. The spec text is amended in this ADR; spec.md update follows in a separate documentation pass.

## References

- [Nix Pills §6 — Modules and merging](https://nixos.org/guides/nix-pills/modules.html)
- [Home Manager — `home-manager.users.<name>._module.freeformType`](https://nix-community.github.io/home-manager/options.xhtml)
- ADR-0011 (HM module scope; constrains where module can read from)
- ADR-0024 (AT-SPI substrate; runtime fallback is the safety net for missing a11y bus)
- spec 004 §FR-015 (amended by this ADR)
