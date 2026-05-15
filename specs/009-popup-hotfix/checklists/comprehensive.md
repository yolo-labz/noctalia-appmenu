# Comprehensive quality checklist — 009-popup-hotfix (v1.0.1)

**Purpose:** "Unit tests for English" — validates the **quality** of `spec.md` + `plan.md` + `data-model.md` + `contracts/*` + `quickstart.md` against requirements-engineering best practice. Does NOT test the implementation.
**Created:** 2026-05-15
**Validates:** `specs/009-popup-hotfix/{spec.md, plan.md, research.md, data-model.md, contracts/*, quickstart.md}`
**Items:** 45
**Mode:** static review against the spec — does NOT verify code or runtime behaviour.
**Synthesised from:** INCOSE GtWR v4 (C1–C12), Wiegers SRS checklist (Correct, Feasible, Necessary, Unambiguous, Verifiable, Complete, Consistent), Volere quality-gateway snow-card, Google SRE Launch Coordination Checklist, GitHub Spec-Kit "unit tests for English" guidance (2026), Kiro 2026 bug-fix spec pattern (extends → preserves → narrows scope), SLSA v1.1, OpenSSF maintainer evaluation, Qt6 a11y bridge community notes.

---

## 1. Requirement Clarity (INCOSE C1–C5, Wiegers "Unambiguous")

- [ ] CHK001 Are all FRs free of banned subjective words (`fast`, `robust`, `seamless`, `efficient`, `intuitive`, `user-friendly`, `simple`, `properly`, `correctly`, `appropriate`)? [Clarity] [Spec §Functional requirements FR-001..008]
- [ ] CHK002 Does every FR carry exactly one normative verb (`MUST` / `MUST NOT` / `SHOULD`) AND is `SHOULD` reserved for documented escape hatches with an inline rationale? [Clarity] [Spec §FR-001..008]
- [ ] CHK003 Is the term "wrapper" defined precisely on first use (`MENU_ITEM → MENU(empty label) → [items]` shape, not just "the Qt thing")? [Clarity] [Spec §FR-001; contracts/recursive-flatten.md §Behavioural contract]
- [ ] CHK004 Is "depth ≥ 2 cascade" expressed identically across spec / plan / contracts (same numeric anchor, same starting-index convention)? [Clarity] [Spec §Scenarios 2; data-model.md §Submenu cascade depth; contracts/popup-surface.md §Layer-shell namespace]
- [ ] CHK005 Is "freezes" reframed as a precise behavioural symptom in every place it appears (not the user-quoted word, but its mechanical cause: "bar input absorbed by full-screen popup MouseArea")? [Clarity] [Spec §Why ¶1; FR-002]
- [ ] CHK006 Is the screen-coordinate-vs-window-coordinate distinction stated in every place where popup positioning is described (no implicit assumption that two layer-shell PanelWindows share an origin)? [Clarity] [Spec §FR-002; research.md §Decision 2; contracts/popup-surface.md §Surface position]

## 2. Requirement Completeness — defect-mode coverage (INCOSE C9)

- [ ] CHK007 Are all 7 confirmed bugs from the four-agent investigation mapped 1-to-1 to an FR (each bug has at least one corresponding FR-NNN)? [Completeness] [Research synthesis vs. Spec §FR-001..008]
- [ ] CHK008 Is the 1 "suspected" bug (FR-008 self-clear latch) explicitly marked as suspected-in-research and elevated-to-required-in-spec, or downgraded with rationale? [Completeness] [Research §Decision 8; Spec §FR-008]
- [ ] CHK009 Are all four refuted hypotheses from the agent reports listed somewhere (research.md §Alternatives, or spec §Out-of-scope) so a reader doesn't relitigate them? [Completeness] [Research §Decision 2 alternatives; verify hover-open, hasChildren race, wl_seat grab, ExclusionMode are each explicitly disposed of]
- [ ] CHK010 Is the multi-toolkit verification matrix enumerated (Qt6 confirmed; what about GTK4, Electron, Tk)? [Completeness] [Spec §SC-006; verify GTK4 either covered or explicitly out-of-scope]
- [ ] CHK011 Are the FIVE QML files touched by the hotfix all named explicitly in `plan.md §Affected files` (AppmenuPopupWindow, SubmenuPopup, BarWidget, MenuRow, plus the two new test harnesses)? [Completeness] [Plan §Lane Q]
- [ ] CHK012 Are the THREE bridge files touched all named explicitly (atspi.rs, active.rs, focus.rs) and the new fixture + test? [Completeness] [Plan §Lane B]

## 3. Requirement Completeness — supply-chain & release (SLSA + SBOM + attestation)

- [ ] CHK013 Is the v1.0.1 release vehicle named explicitly with the existing supply-chain spine preserved (CycloneDX 1.6 per ADR-0026, attest-build-provenance v4.1.0, SOURCE_DATE_EPOCH)? [Completeness] [Spec §SC-008; Plan §Approach ¶3]
- [ ] CHK014 Is the "no re-tag" invariant restated for the v1.0.1 cut (or explicitly inherited from constitution VI)? [Completeness] [Plan §Rollout; Constitution §VI]
- [ ] CHK015 Is `gh attestation verify` named as the SC-008 acceptance command (not just "release workflow succeeds")? [Measurability] [Spec §SC-008]
- [ ] CHK016 Is the order-of-operations for the release explicit (Lane Q merges → Lane B merges → Cargo bump → tag → verify → flake.lock bump → `nh os switch`)? [Completeness] [Plan §Rollout; quickstart.md §Release walk-through]

## 4. Requirement Consistency (INCOSE C9 Consistent sets)

- [ ] CHK017 Do the eight FRs in spec.md map exactly to the eight design decisions in research.md (no orphan decision, no FR without a decision)? [Consistency] [Spec §FR-001..008 vs. Research §Decision 1..8]
- [ ] CHK018 Does `plan.md §Architecture sketch` show every FR's owning component (no FR floats homeless; no component holds an FR not in spec)? [Consistency] [Plan §Architecture; Spec §FRs]
- [ ] CHK019 Is the term `focused_output` used identically across spec, plan, data-model, and contracts (never aliased to `focused_screen` or `output_name`)? [Consistency] [Spec §FR-006; data-model.md; contracts/active-json-schema.md]
- [ ] CHK020 Are the Test contracts cited inline in each FR consistent with the test files named in plan.md §Affected files (no test contract references a file that the plan does not create)? [Consistency] [Spec §FR-001..008 Test contracts vs. Plan §Lane Q + Lane B test files]
- [ ] CHK021 Is FR-002 "constrained popup surface" consistent with NFR-002 "no new wl_seat grab" — i.e. the surface change does not require switching to PopupWindow? [Consistency] [Spec §FR-002 vs. §NFR-002; research.md §Decision 2 alternatives]
- [ ] CHK022 Does `_sameTopLevel` widening (FR-005) NOT contradict the PR #51 anti-flicker invariant cited in BarWidget.qml (i.e. structural-change comparisons stay cheap)? [Consistency] [Spec §FR-005; Plan §Risk 3; research.md §Decision 5]

## 5. Acceptance Criteria Quality (Wiegers "Verifiable" + Volere fit criterion)

- [ ] CHK023 Does every SC reference an observable artefact (CI job name, command output, screenshot diff, attestation exit code, fixture round-trip) — not a subjective experience? [Measurability] [Spec §SC-001..008]
- [ ] CHK024 Does SC-001's "screenshot diff" specify the diff tool and the tolerance (pixel count, perceptual hash, exact-match)? [Measurability] [Spec §SC-001]
- [ ] CHK025 Does SC-006's "three-of-five Qt6 apps" enumerate the five candidates AND name a tiebreaker if exactly three pass? [Measurability] [Spec §SC-006]
- [ ] CHK026 Does SC-005's "qmltest harness opens a depth-3 cascade" specify what counts as "opens" (window mapped, content rendered, visible == true on every depth)? [Measurability] [Spec §SC-005; contracts/popup-surface.md §Test contracts]
- [ ] CHK027 Does SC-008 declare a positive AND a negative gate (binary attests-verify exit 0 AND no-other-attestation regressions)? [Measurability] [Spec §SC-008]

## 6. Scenario Coverage (INCOSE C10 Complete; Wiegers "Complete")

- [ ] CHK028 Is there a scenario covering "user clicks the same top-level button twice in succession" (toggle vs. re-open semantics)? [Coverage] [Gap — verify against Spec §Scenarios]
- [ ] CHK029 Is there a scenario covering "popup is open and the focused app emits `accessible-children-changed` mid-render"? [Coverage] [Gap — relates to FR-005 dedup but not in §Scenarios]
- [ ] CHK030 Is there a scenario covering "popup is open and the user presses Escape"? [Coverage] [Gap — research.md §Decision 2a sub-decision mentions Esc dismissal but not in §Scenarios]
- [ ] CHK031 Is there a scenario covering "user opens a deep cascade (depth ≥ 3), then alt-tabs to another app"? [Coverage] [Gap — combines focus change + open submenu chain]
- [ ] CHK032 Is the "bridge slow / app slow / AT-SPI fetch timeout" path covered as either an FR or an explicit out-of-scope (current spec leaves it implicit)? [Coverage] [Gap — relates to NFR-003 budget but no scenario]

## 7. Edge Case Coverage (Wiegers + INCOSE C12 Validatable)

- [ ] CHK033 Are all four wrapper-flatten edge cases enumerated (multi-child wrapper rejected; nested wrapper handled bottom-up; empty leaf untouched; toggle/radio leaves preserve `toggle_state`)? [Coverage] [Contracts/recursive-flatten.md §Edge cases]
- [ ] CHK034 Are the popup-surface edge cases enumerated (anchor item near right edge → popup shifts left; anchor item near bottom → popup shifts up; multi-monitor anchor handoff)? [Coverage] [Gap — popup-surface.md §Surface position alludes but doesn't enumerate]
- [ ] CHK035 Is the `focused_output: null` case (compositor restart, unrouted toplevel) explicitly covered in the QML consume side (not just the bridge produce side)? [Coverage] [Spec §FR-006; data-model.md §focused_output validation rule]
- [ ] CHK036 Is the `Loader.status === Loader.Error` branch handled (component fails to instantiate)? [Coverage] [Data-model.md §Loader.status state transitions; verify behaviour beyond log+return]

## 8. Non-Functional Requirements (INCOSE C11 Feasible)

- [ ] CHK037 Is each NFR feasible to verify in CI vs. in manual smoke — and is the verification location explicit per NFR? [Measurability] [Spec §NFR-001..004]
- [ ] CHK038 Is NFR-001 "no render regression at idle" tied to a specific test (existing flicker test suite, qmltest run, manual scrub against AMD GPU)? [Measurability] [Spec §NFR-001]
- [ ] CHK039 Does NFR-003 "recursive flatten budget" name the units (microseconds per item, total milliseconds per fetch) and the host class? [Clarity] [Spec §NFR-003; contracts/recursive-flatten.md §Test contracts §Performance]
- [ ] CHK040 Does NFR-004 "backwards-compatible JSON schema" cite the concrete consumer behaviour required (default value, no-throw on absent field, schema version unchanged)? [Completeness] [Spec §NFR-004; contracts/active-json-schema.md §Versioning]

## 9. Dependencies, Assumptions, and Risks (Volere snow-card + Google SRE)

- [ ] CHK041 Does every external dependency named in `plan.md §Constraints` carry a single version constraint (no contradiction between spec.md and plan.md, no implicit "latest")? [Consistency] [Spec §Constraints; Plan §Approach]
- [ ] CHK042 Are the SIX risks in plan.md each paired with a concrete mitigation that points to a specific test or code path (not a vague "we'll be careful")? [Completeness] [Plan §Risks 1..6]
- [ ] CHK043 Is the self-hosted runner availability assumption explicit AND paired with a fallback (defer-merge, retry, manual smoke)? [Completeness] [Spec §Constraints; Plan §Risk 6]

## 10. Constitution alignment (per project §V — speckit-driven)

- [ ] CHK044 Does the Constitution Check table in plan.md grade EVERY principle (I–VII) with PASS/FAIL/N/A AND a one-line note (no blank cells, no implicit PASS)? [Consistency] [Plan §Constitution Check; Constitution §I–VII]
- [ ] CHK045 If any principle is graded FAIL, is it justified inline with either an ADR amendment, a constitution amendment proposal, or an explicit one-off exception agreed by `@phsb5321` (per template instruction)? [Consistency] [Plan §Constitution Check; Spec-template.md instruction]

---

## How to work through this checklist

1. **Sit with the spec open.** Each item references a section by `[Spec §...]` / `[Plan §...]` / `[Contracts §...]` / `[Research §...]` / `[Data-model §...]`, or marks a `[Gap]`. Open the cited section and read the surrounding requirement before ticking.
2. **Tick = passes, unticked = open.** Do not partial-tick; a partial pass goes in the Notes section below with the specific defect.
3. **`[Gap]` items are spec changes**, not implementation defects — they trigger a `docs(speckit): spec 009 v1.0.1` patch commit on `009-popup-hotfix`.
4. **`[Ambiguity]` items get a 1-sentence clarification** added to the cited section.
5. **`[Conflict]` items need a constitution check** — escalate to `@phsb5321`.
6. **Cap at 3 review iterations** per the project's quality-gate convention; surviving items become explicit open questions in `plan.md §Open questions`.

## Coverage tally (against industry standards)

- **INCOSE GtWR v4 characteristics referenced:** C1 (Necessary), C2 (Appropriate), C3 (Unambiguous), C4 (Complete), C5 (Singular), C9 (Consistent), C10 (Complete sets), C11 (Feasible), C12 (Validatable) — 9 of 13 characteristics directly tested.
- **Wiegers SRS qualities addressed:** Correct, Feasible, Necessary, Unambiguous, Verifiable, Complete, Consistent — 7 of 10 (Modifiable, Prioritized, Traceable are inherent to the speckit workflow).
- **Volere snow-card gates:** Completeness (CHK007–CHK012), Measurability (CHK023–CHK027), Traceability (CHK017–CHK020), Consistency (CHK017–CHK022), Fit criterion (CHK024).
- **Google SRE Launch Coordination parallels:** Launch readiness (CHK013–CHK016), failure response (CHK008, CHK032), rollback (CHK016 — `nh os switch`/Nix-generation pin path).
- **GitHub Spec-Kit 2026 "unit tests for English":** every item asks about WRITTEN content quality, not runtime behaviour.
- **Kiro 2026 bug-fix spec pattern:** extends-not-rewrites scope (CHK009 — refuted hypotheses preserved); preserves architectural decisions (CHK021 — NFR-002 wl_seat invariant); narrowed surface (CHK010 — toolkit matrix bounded).

## Traceability stats

- **Items with explicit traceability reference:** 41 / 45 = **91%** (target ≥ 80%, Spec-Kit 2026 §traceability requirement met).
- **`[Gap]` markers:** 7 (CHK028, CHK029, CHK030, CHK031, CHK032, CHK034, CHK010 latent).
- **`[Ambiguity]` markers:** 0 (none flagged in this drafting pass).
- **`[Conflict]` markers:** 0 (no constitutional conflict detected).

## Notes (fill during review)

> Reviewer: _______________
> Date: _______________
>
> CHK### — defect summary — proposed fix
> ...

## Open questions surfaced during this drafting

1. Should the multi-toolkit matrix (CHK010) be promoted from a checklist gap to a spec-level FR, or is its current implicit handling (ADR-0024 covers Qt + GTK; spec 009 scopes to Qt6) sufficient?
2. The Esc-dismissal scenario (CHK030) is a known v1.x gap (constitution out-of-scope §Mnemonics + keyboard nav). Should this hotfix's popup-surface change at least leave Esc as a TODO, or stay completely silent?
3. CHK032 (AT-SPI fetch timeout) was a Lane B suspected bug per the agents — is the v1.0.0 3-second `FETCH_BUDGET` constant carried verbatim into v1.0.1, or is it now a configurable that this spec should mention?
