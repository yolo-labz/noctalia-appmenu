# ADR-0025 — Cognitive-complexity waiver for `find_app_for_pid` + `fetch_menu_tree`

Status: Accepted (time-boxed)
Date: 2026-05-12
Supersedes: —
Superseded by: —

## Context

The v1.0.0 SonarQube quality gate (FR-026, spec 004-project-completion) sets a hard ceiling of **cognitive complexity ≤ 15 per function** for the bridge. Today, two functions in `bridge/src/atspi.rs` exceed it:

| Function | Lines | Estimated complexity | Source |
|---|---|---|---|
| `find_app_for_pid` | `bridge/src/atspi.rs:295–408` | 18–22 | `specs/004-project-completion/research.md` §6 — Sonar audit |
| `fetch_menu_tree` | `bridge/src/atspi.rs:618–746` | 16–20 | same |

Both functions are load-bearing for the AT-SPI substrate (ADR-0024) and were grown to their current shape iteratively as the v0.3 AT-SPI rollout exposed real-world quirks (Anki Python-wrapper PID mismatch; KDE `org.kde.<app>` double-prefix `app_id` normalisation; GTK4 `GtkPopoverMenuBar` empty-children fallback; Qt6 `toggle_state` extraction; sub-menu DFS recursion).

The Sonar gate as configured would block the `v1.0.0` tag today. Refactoring either function below 15 is non-trivial: each branch in `find_app_for_pid` corresponds to a distinct matching strategy (PID-direct, fuzzy-name, app-id round-trip), and `fetch_menu_tree`'s complexity is dominated by the cross-toolkit property fetch (each property has a typed AT-SPI accessor with its own error path).

Spec 004 FR-027 explicitly permits either path:

> `find_app_for_pid` and `fetch_menu_tree` in `bridge/src/atspi.rs` are refactored to bring cognitive complexity below 15 each, OR an ADR (ADR-0025) documents the deviation with rationale before `v1.0.0` ships.

Lane A (`specs/005-bridge-completion`) owns `atspi.rs` and may take the refactor path. Lane D (`specs/008-ci-quality-docs`) cannot refactor `atspi.rs` without violating its lane boundary, so it lands this ADR as the fallback path: a **time-boxed waiver** that lets the v1.0.0 quality-gate snapshot stay green while Lane A's refactor merges, and expires if Lane A's refactor lands or at v1.0.1, whichever is first.

## Decision

1. Both `find_app_for_pid` and `fetch_menu_tree` in `bridge/src/atspi.rs` are exempt from the Sonar `sonar.rust.cognitive.maximumComplexityPerFunction=15` threshold for the duration of the v1.0.0 release.
2. The exemption is recorded in the SonarQube UI as an "Issue rule deviation" against the two function symbols (server-side, applied out-of-band by Pedro when the v1.0.0 tag is cut).
3. The exemption is **time-boxed**:
   - **Trigger A:** Lane A's refactor PR (under spec 005) brings both functions below complexity 15. The waiver is removed in the same PR that merges the refactor.
   - **Trigger B:** v1.0.1 ships. The waiver is removed; if the refactor still hasn't landed, the v1.0.1 cycle must take the work or the project re-cuts the ADR with a new expiry.
   - Whichever trigger fires first ends the waiver.
4. No other function in `bridge/src/**` is exempted. New functions must land below complexity 15 — this ADR is not a blanket waiver.

## Consequences

- **Positive:** Unblocks the v1.0.0 tag from a gate that punishes lane-boundary discipline (Lane D's refactor would constitute a Lane A invasion). The quality-gate snapshot stays meaningful — only two named functions are exempted, every other code path is governed.
- **Positive:** Records the technical-debt cost of the iterative AT-SPI rollout explicitly. Pedro can plan the refactor against `bridge/src/atspi.rs` in a Lane A follow-up with full context.
- **Negative:** Two of the most-touched functions in the bridge are temporarily ungoverned for complexity. Mitigation: every PR that touches either function is reviewed against the refactor goal — Lane A's spec 005 already lists the refactor as in-scope.
- **Negative:** Sonar UI rule deviations are server-side and not directly visible in the repository. Mitigation: this ADR is the authoritative record; the Sonar UI mirrors it.

## Alternatives considered

- **Refactor under Lane D.** Rejected — violates Lane A's boundary (`bridge/src/**` is Lane A's exclusive surface per spec 004 §Architecture sketch + Lane Allocation). A cross-lane edit creates merge conflicts and breaks the parallel-worker pattern that lane-splits the v1.0.0 roadmap.
- **Raise the threshold to 25 globally.** Rejected — loses governance over every other function in the bridge. The current 15-ceiling has caught real complexity creep (e.g. `niri::handle_event` was refactored in PR #60 because Sonar flagged it). A narrow per-function exemption preserves the gate's signal.
- **Disable the cognitive-complexity rule entirely.** Rejected — same loss of governance; precedent for blanket-disabling Sonar rules makes future bumps harder.
- **Defer v1.0.0.** Rejected — the constitution's v1 ship gate ("niri Qt+GTK works on three different apps, integration tests pass on CI runner, README's 'Verify the install' recipe works clean on a fresh NixOS box") is met today; gating the tag on a complexity refactor that has no user-visible impact would delay the v1 milestone for code-quality reasons that the waiver path addresses adequately.

## Verification

- The Sonar quality-gate report on the v1.0.0 merge commit shows the cognitive-complexity threshold met for every function except the two named here.
- The two named functions are listed in the Sonar UI as "Deviation accepted (ADR-0025)" or equivalent annotation, linkable to this file.
- A Lane A follow-up issue (`#TBA`) tracks the refactor; when it merges, the deviation is removed and this ADR's status flips to "Superseded by <commit-sha>".

## Expiration

This ADR expires when one of:

- The refactor of `find_app_for_pid` AND `fetch_menu_tree` lands on `main` (both must come below complexity 15). The same PR removes the Sonar deviations and updates this ADR's Status to "Superseded".
- The `v1.0.1` tag is cut. At that point the waiver is removed regardless of refactor state; if the refactor still hasn't landed, the v1.0.1 cycle takes the work, or a new ADR re-cuts the waiver with a fresh expiry.

The first of those events terminates the waiver.

## References

- `specs/004-project-completion/spec.md` — FR-026 (v1 Sonar gate), FR-027 (refactor-or-waiver dichotomy).
- `specs/004-project-completion/research.md` §6 — Sonar audit, complexity estimates.
- `specs/008-ci-quality-docs/spec.md` — Lane D's surface, why this ADR lives here.
- `specs/005-bridge-completion/` — Lane A's spec (refactor path; owns `atspi.rs`).
- `docs/adr/ADR-0024-atspi-substrate.md` — AT-SPI substrate decision that drove the complexity growth.
- `sonar-project.properties` — `sonar.rust.cognitive.maximumComplexityPerFunction=15` (the threshold this waiver narrows).
