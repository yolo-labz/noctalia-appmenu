# ADR-0005 — niri-only in v1

Status: Accepted
Date: 2026-05-04

## Context

Hyprland and Sway users will eventually want this. Designing the focus-pid bridge as a compositor-agnostic abstraction *before* shipping any working implementation produces wrong abstractions — we don't yet know what the abstraction's contract should be.

## Decision

v1 ships niri support only. The bridge's focus oracle is `niri-ipc`. No abstract `FocusOracle` trait, no compositor enum, no runtime detection. A Hyprland branch is welcome via PR in v2 — and we expect *that* PR to drive the abstraction shape, not a hypothetical pre-design.

## Consequences

- **Positive:** Smaller v1. No premature abstractions. Dramatically lower test matrix.
- **Negative:** Hyprland users wait. Marketing footprint smaller.
- **Mitigation:** README leads with "niri only" so expectations are set. Pinned `[requires-niri]` GitHub issue label tracks v2 demand.

## Alternatives considered

- **Generic `FocusOracle` trait now:** Rejected per "premature abstraction" principle.
- **Best-effort compositor auto-detection:** Adds boot-time complexity; without a Hyprland implementation, the auto-detect branch is dead code. Rejected.

## References

- See "Premature abstractions" discussions across multiple `~/NixOS/specs/` retros.
