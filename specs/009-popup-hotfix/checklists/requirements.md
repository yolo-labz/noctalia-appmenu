# Requirements quality checklist — spec 009-popup-hotfix

Spec: `specs/009-popup-hotfix/spec.md`
Checklist run: 2026-05-15

## Content quality

- [x] Spec describes WHAT and WHY, not HOW (no concrete code, no
      implementation choices in scenarios / FRs).
- [x] Written for stakeholders (Pedro + future contributors), not
      developer-only jargon. Architectural references (ADR-0008/0024)
      are footnotes, not load-bearing prose.
- [x] Section order matches `.specify/templates/spec-template.md`.
- [x] Mandatory sections present (Why, User scenarios, Functional
      requirements, Success criteria). Optional sections kept
      relevant.
- [x] No checklists embedded inside the spec itself (this file is the
      separate validation artefact, per command spec).

## Requirement completeness

- [x] Each FR is finitely verifiable — every FR-NNN has an explicit
      "Test contract" sentence pointing at the pass/fail check.
- [x] No `[NEEDS CLARIFICATION]` markers remain. (Pedro's symptoms +
      the four-agent synthesis disambiguated every scope decision; no
      open questions of high-impact remain.)
- [x] Non-functional requirements cover performance (NFR-003), no-
      regression (NFR-001), protocol invariants (NFR-002),
      schema-compat (NFR-004).
- [x] Out-of-scope section explicitly excludes hover-open, mnemonics,
      multi-compositor, AT-SPI replacement, envelope refactor.
- [x] Constraints / dependencies enumerate runner availability, crate
      pins, Quickshell version, harden-runner egress, lefthook /
      conventional commits / DCO.

## Feature readiness

- [x] User scenarios cover the failure modes Pedro reported (top-level
      width clamp, depth-≥-2 cascade silently dropped, bar-click
      absorbed by popup, stale post-refresh menu, empty-menu
      collapse).
- [x] Success criteria are measurable: each SC names a specific
      verification (manual smoke, fixture round-trip, qmltest harness,
      attest-verify exit code, three-of-five matrix pass).
- [x] Success criteria reference apps from the verification matrix,
      not just "any Qt6 app".
- [x] Spec implies a single-PR-per-defect implementation pattern that
      fits the constitution's ≤ 25-task ceiling without effort.
- [x] Release vehicle named (v1.0.1) and tagged to the existing
      v1.0.x supply-chain spine (ADR-0026 CycloneDX 1.6).

## Constitution alignment

- [x] **Principle I (niri-only)** — spec scopes to niri only;
      Hyprland / KWin out of scope.
- [x] **Principle II (sidecar bridge by default)** — bridge owns
      AT-SPI walker change (FR-001); QML owns presentation +
      hit-testing (FR-002..005, 007, 008). No new D-Bus surface; no
      QML-side bus claim.
- [x] **Principle III (worktree-first)** — spec authored in
      `noctalia-appmenu-009-popup-hotfix` worktree, not in main.
- [x] **Principle IV (Conventional Commits + DCO)** — implementation
      PRs will follow `fix(qml):` / `fix(bridge):` per FR.
- [x] **Principle V (speckit-driven)** — this IS the spec.
- [x] **Principle VI (release-engineering compliance)** — SC-008
      preserves attest + SBOM + reproducible-build chain; no workflow
      changes required.
- [x] **Principle VII (graceful degradation)** — FR-006 + FR-008 keep
      the widget honest under partial-data conditions.

## Iteration log

- 2026-05-15 17:42 — initial draft, all checklist items pass on first
  pass. No `[NEEDS CLARIFICATION]` markers were generated; the
  four-agent investigation supplied enough ground truth to fill every
  ambiguity with a defensible default.

## Status

**PASS — spec is ready for `/speckit:plan`.**
