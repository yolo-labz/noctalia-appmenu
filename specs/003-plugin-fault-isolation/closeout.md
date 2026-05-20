# Spec 003 close-out

**Status:** closed
**Closed:** 2026-05-20
**Shipped via:** PRs #51, #52, #55, #57, #59, #60, #74

## Disposition

Spec 003 codified the fault-isolation contract: appmenu bugs must not
escape the appmenu's bar slot. All FRs shipped:

- **Input-grab containment** — PR #52 (`xdg_popup.grab(wl_seat)`).
- **Surface-damage containment** — PR #51 (noctalia v4 single-shared
  PanelWindow surface workaround).
- **Isolation envelope around applySnapshot** — PR #57 (defer via
  `Qt.callLater`).
- **active.json schema v1 + producer-side dedup** — PR #59.
- **niri event-stream fixture-replay test harness** — PR #60.
- **Nested submenus + toggle_state + icon_name + screen guard** —
  PR #74 (subsumed Lane B work from spec 006).
- **Speckit doc landing** — PR #55.

Isolation contract has held across v0.3.x → v1.0.23. Self-heal
cascade (spec 015 FR-006) is the latest reinforcement.

## Successor specs

- **004** — v1.0.0 project completion umbrella — closed.
- **006** — Plugin completion (Lane B) — closed.
- **015** — Ship-ready completion (self-heal cascade) — mechanically
  done, awaiting Pedro SC-002 visual signoff.

## Why this doc exists

Speckit-pipeline audit consistency. No code change. No follow-up tasks.
