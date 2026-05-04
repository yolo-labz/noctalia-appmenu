# ADR-0011 — Home-Manager submodule, not flake module

Status: Accepted
Date: 2026-05-04

## Context

This is per-user state: a noctalia plugin lives at `~/.config/noctalia/plugins/`, the bridge runs as `systemd --user`, environment variables (`QT_QPA_PLATFORMTHEME`, `GTK_MODULES`) live in the user session. None of this belongs at the system level.

## Decision

Distribute as a Home-Manager submodule under `programs.noctalia.plugins.appmenu`. Expose:

- `enable` — boolean.
- `package` — derivation override.
- `bridge.package` — bridge derivation override.
- `bridge.config` — TOML attrset merged into the bridge's config file.
- `registrar` — enum: `"vala-panel"` (default), `"none"` (user provides their own).
- `hideInWindowMenubar` — boolean. Sets `QT_QPA_PLATFORMTHEME=appmenu-qt5` and `GTK_MODULES=appmenu-gtk-module` in the session environment.
- `widgetPlacement` — string mirroring noctalia's bar widget placement (`"left"`, `"center"`, `"right"`). Default: `"left"`.

## Consequences

- **Positive:** Clean per-user config. Plays nice with Pedro's existing `programs.noctalia` patterns in `~/NixOS/modules/home/linux/noctalia/shell.nix`.
- **Negative:** No system-level config story for shared multi-user machines. Acceptable — multi-user Linux desktops with niri are rare.
- **Mitigation:** A `flake-modules/system.nix` placeholder is reserved for v2 if demand surfaces.

## Alternatives considered

- **NixOS module:** Mismatched scope (per-user state, not system).
- **Pure flake-package consumed via `home.packages`:** Loses the typed option surface that catches misconfiguration at evaluation time.

## References

- `~/NixOS/modules/home/linux/noctalia/shell.nix` — pattern this submodule integrates with.
