# Spec 007 close-out (Lane C)

**Status:** closed
**Closed:** 2026-05-20
**Shipped via:** PRs #75, #80

## Disposition

Spec 007 was Lane C of the v1.0.0 four-lane split (umbrella spec 004).
Scope: Nix surface completion — AT-SPI prerequisites, flake hygiene,
HM module rewrite for AT-SPI world.

All FRs shipped:

- **#75** — AT-SPI prerequisites + flake hygiene (Lane C core):
  `QT_ACCESSIBILITY=1`, `services.gnome.at-spi2-core.enable` wired,
  `vala-panel-appmenu` removed, version derivation made deterministic.
- **#80** — Drop `osConfig` from HM module (eval recursion, ADR-0027).

NixOS system module mirror remained deferred to v2 per ADR-0011 (HM-only).

## Successor specs

None pending. NixOS system module mirror is a future v2 line item.

## Why this doc exists

Speckit-pipeline audit consistency. No code change. No follow-up tasks.
