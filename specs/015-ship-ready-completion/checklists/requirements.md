# Requirements Quality Checklist — spec 015 ship-ready completion

**Source:** `specs/015-ship-ready-completion/spec.md`
**Run by:** `/speckit:specify` validation phase
**Iteration:** 1 (initial)

## Content quality

- [x] Focuses on user-visible WHAT and WHY, not implementation HOW.
      *Evidence:* User scenarios describe Pedro's flow, not Rust struct
      changes. Implementation hints are confined to FR descriptions
      (acceptable — they reference *verification* methods).
- [x] Written for stakeholders (Pedro, future Claude sessions, code
      reviewers), not implementation-detail engineers.
- [x] No raw code blocks in user scenarios.
- [x] Each section header from `spec-template.md` is present.

## Requirement completeness

- [x] Each FR is finitely verifiable with a concrete pass/fail check.
- [x] Each NFR carries a metric (latency ms, trial count, count of
      FAIL rows).
- [x] User scenarios use Given/When/Then.
- [x] Out-of-scope section is concrete (named items, not "future work").
- [x] Constraints reference verifiable upstream artefacts (shell
      store path, niri-ipc crate version, settings keys).
- [x] Assumptions are documented; reasonable defaults explained.

## Feature readiness

- [x] Success criteria are measurable AND technology-agnostic in
      outcome (trial count, FAIL row count, journal-line count).
- [x] Pre-merge gates referenced (`scripts/release.sh` extension,
      pre-commit hook extension) are concrete files in the repo.
- [x] Failure modes are tied back to the v1.0.5..v1.0.21 case study
      so future Claude sessions inherit the lessons.
- [x] Cross-references resolved to file paths or PR numbers.

## Clarification markers

- [ ] **0 NEEDS CLARIFICATION markers remain.** Verified.
      *(Initial draft contains zero markers — informed defaults
      filled every gap.)*

## Validation result

**PASS** — All quality items satisfy the checklist. Spec is ready
for `/speckit:clarify` (which will surface any latent ambiguity)
followed by `/speckit:plan`.

## Notes for next phase

- `/speckit:clarify` should probe FR-003 (accelerator-key fallback
  scope), FR-007 (release-gate fixture environment), and FR-008
  (symptom-phrase matching tolerance).
- `/speckit:checklist` should generate four downstream checklists
  per FR-007: visual-parity, routing-smoke, self-heal-absence,
  deploy-idempotence.
