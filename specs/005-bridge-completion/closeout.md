# Spec 005 close-out (Lane A)

**Status:** closed
**Closed:** 2026-05-20
**Shipped via:** PRs #77, #78, #79, #80, #81

## Disposition

Spec 005 was Lane A of the v1.0.0 four-lane split (umbrella spec 004).
Scope: bridge completion — focus tracker, AT-SPI walker, registrar
ownership, version bump to v1.0.0.

All FRs shipped:

- **#77** — Focus tracker + AT-SPI walker (Lane A core).
- **#78** — Bridge v1.0.0-rc.1 bump.
- **#79** — CycloneDX 1.6 emission (syft constraint, ADR-0026).
- **#80** — Drop `osConfig` from HM module (eval recursion, ADR-0027).
- **#81** — Bridge v1.0.0 final bump.

Lane A acceptance criteria all met at v1.0.0 release (12-13/05/2026).

## Successor specs

- **013** — SOTA overhaul (eager-walk + atspi-signal residue) — plan
  PR #122 awaiting Pedro R1/R2/R3 dispositions.
- **015** — Ship-ready completion (routing reliability, gates) —
  mechanically done.

## Why this doc exists

Speckit-pipeline audit consistency. No code change. No follow-up tasks.
