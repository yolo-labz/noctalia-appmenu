# Spec 001 close-out

**Status:** closed
**Closed:** 2026-05-20
**Shipped via:** v0.1.0 release train (initial scaffold + follow-up PRs)

## Disposition

Spec 001 was the MVP scope for global-menu rendering on niri. All FRs
shipped via the v0.1.0 release train:

- Bridge + plugin + CI/CD + speckit scaffolding — commit `ca7b83d`
  (initial scaffold, pre-PR-numbering).
- Anki menubar in top bar — shipped v0.1.0; demonstrated against Anki
  with `appmenu-registrar` + `noctalia-appmenu-bridge`.
- Focus-following with ≤ 200 ms debounce + render — shipped v0.1.0
  (debounce later revised in spec 015 FR-001 to 150 ms).
- Out-of-scope items (Firefox/Electron) stayed out per constitution.

## Successor specs

- **002** — Bridge DBusMenu mirror (v0.2 surface) — closed.
- **003** — Plugin fault-isolation envelope — closed.
- **004** — v1.0.0 project completion umbrella — closed.

## Why this doc exists

Spec 001 predates the formal speckit close-out artifact. This file
brings 001's paperwork into the same shape as later specs (002-008,
015) for consistency in the speckit-pipeline audit.

No code change. No follow-up tasks.
