# Spec quality checklist — 004-project-completion

**Validated against:** `specs/004-project-completion/spec.md` (initial draft, 2026-05-12)
**Iteration:** 1 of 3

Mark each item ✅ pass / ❌ fail / ⚠️ partial. Document specific issues with line citations.

## Content quality

- [x] **CQ-001** Spec answers "what does completion mean?" without prescribing implementation tactics. ✅ — Why + scenarios + FRs describe outcomes, not code shapes.
- [x] **CQ-002** Spec is readable by a non-Rust / non-QML stakeholder. ✅ — Avoids C++/zbus/QML jargon outside the entity table.
- [x] **CQ-003** Why section motivates the work and names the stakeholders. ✅ — Pedro + future contributors + fresh-NixOS users named.
- [x] **CQ-004** Out-of-scope section explicitly lists deferred items with reason. ✅ — 9 items, each with a deferral target (v2 / never / separate spec).

## Requirement completeness

- [x] **RC-001** Every functional requirement is testable. ✅ — Each FR has a pass/fail criterion (test name, behaviour, log/metric).
- [x] **RC-002** Every non-functional requirement is measurable. ✅ — NFRs cite percentiles, byte-equality, time bounds, log destinations.
- [x] **RC-003** Every user scenario follows Given/When/Then. ✅ — 7 scenarios, all 3-clause form.
- [x] **RC-004** Each success criterion is verifiable without implementation details. ✅ — SC-001..SC-008 reference observable outcomes or CI jobs.
- [x] **RC-005** Key entities listed with their owners + locations. ✅ — 8 entities, each mapped to file or interface.
- [x] **RC-006** Constraints / dependencies are explicit and versioned. ✅ — Quickshell ≥ 0.3.0, noctalia-shell ≥ 1.0.0, niri IPC-1.x, at-spi2-core ≥ 2.50, Qt6 ≥ 6.7, GTK4 ≥ 4.14.
- [x] **RC-007** Assumptions are surfaced separately, not buried in FRs. ✅ — Assumptions section lists 6 items.

## Feature readiness

- [x] **FR-001** Spec maps to a known ship gate. ✅ — Constitution v1.0.0 gate quoted verbatim in §Why and §SC.
- [x] **FR-002** Spec scope fits the constitution's "≤25 tasks per spec" cap. ⚠️ — 29 FRs is over the implementation-task cap. **Mitigation:** spec.md is the umbrella; each FR cluster (focus tracker, AT-SPI, plugin, Nix, CI, Sonar, docs) will spawn its own `005-…` / `006-…` follow-up spec at the `speckit.plan` phase if implementation tasks exceed 25.
- [x] **FR-003** Spec is independently reviewable from past specs. ✅ — Spec 002 supersession via ADR-0024 noted in §Why; spec 003 inheritance noted in §Constraints.
- [x] **FR-004** Spec has zero `[NEEDS CLARIFICATION]` markers. ✅ — Audit produced concrete findings; no ambiguity remained.
- [x] **FR-005** Spec is grounded in real audit findings, not speculation. ✅ — Every FR maps to a `research.md` §1–§7 finding (synthesis table §8).

## Validation summary

**Pass count:** 14 of 15 fully pass, 1 partial.
**Action on partial (FR-002):** Documented mitigation in the checklist row itself. No spec change needed — the constitution's "25 tasks" cap is an *implementation* cap (per Development Workflow §3), not a *requirements* cap. Spec.md is the umbrella; sub-specs ladder down. This is the explicit pattern of the workflow.

**Outstanding `[NEEDS CLARIFICATION]` markers:** 0.

**Iteration verdict:** PASS. Spec is ready for `speckit.plan`.

## Open follow-ups (not blocking)

- After `speckit.plan`, confirm the FR-cluster → sub-spec mapping does not exceed 7 child specs (one per audit lane).
- ADR-0025 may be needed to accept cognitive-complexity deviation if FR-027's refactor is deferred — flag at planning time.
- The 7-day soak test (SC-005) cannot run in CI; document as manual-verification step with sign-off owner.
