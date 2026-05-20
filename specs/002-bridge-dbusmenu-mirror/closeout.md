# Spec 002 close-out

**Status:** closed
**Closed:** 2026-05-20
**Shipped via:** v0.2 surface + ADR-0022 (bridge owns registrar)

## Disposition

Spec 002 scoped the bridge implementing `com.canonical.dbusmenu`
server-side at a constant address so the QML plugin could subscribe
without per-app D-Bus dance. Architecture later evolved past the pure
DBusMenu-mirror pattern (ADR-0023 fetch-on-focus, ADR-0024 AT-SPI
substrate), but the underlying problem — "give the plugin a fixed
proxy" — was solved by the bridge taking ownership of the registrar.

Key PRs:

- **#21** — Plugin BarWidget FileView rewrite (DBusObject not in
  Quickshell).
- **#29** — Bridge owns `com.canonical.AppMenu.Registrar` (ADR-0022).
- Subsequent ADRs **0023 / 0024** moved the data path off pure
  DBusMenu mirroring to AT-SPI eager-walk, but the proxy surface at
  `org.noctalia.AppMenu /org/noctalia/AppMenu/Active` remained intact.

## Successor specs

- **004** — v1.0.0 project completion umbrella — closed.
- **005** — Bridge completion (Lane A) — closed.
- **013** — SOTA overhaul (eager-walk research) — close-out plan PR #122.

## Why this doc exists

Speckit-pipeline audit consistency. Brings 002's paperwork into the
same shape as later specs. No code change. No follow-up tasks.
