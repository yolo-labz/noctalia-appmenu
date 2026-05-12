# Release-readiness checklist — 004-project-completion (v1.0.0)

**Purpose:** Unit-tests-for-English on `spec.md` + `plan.md` + `tasks.md`. Validates the **quality** of the requirements (Completeness, Clarity, Consistency, Measurability, Coverage), not the implementation. Synthesised from INCOSE GtWR v4, Wiegers SRS checklist, Volere quality gateway, Google SRE Launch Coordination Checklist, SLSA v1.1, GitHub artifact attestations, SonarQube quality-gate guidance, OpenSSF maintainer evaluation guide, and Qt AT-SPI bridge community notes (see `research.md` §10 + the web-research swarm dispatched 12/05/2026).
**Created:** 2026-05-12
**Validates:** `specs/004-project-completion/{spec.md, plan.md, tasks.md, data-model.md, contracts/*, quickstart.md}`
**Items:** 42
**Mode:** static review against the spec — does **not** verify code.

---

## Requirement Clarity (INCOSE C1–C5, Wiegers "Unambiguous")

- [ ] CHK001 Are all FRs free of banned subjective words (`fast`, `robust`, `seamless`, `efficient`, `intuitive`, `user-friendly`, `simple`)? [Clarity] [Spec §Functional requirements]
- [ ] CHK002 Is "graceful degradation" replaced everywhere with a quantified fit criterion (e.g., "menubar collapses to zero-paint slot within 2 s of bridge SIGSTOP")? [Clarity] [Spec §NFR-002, §FR-005, §Out-of-scope]
- [ ] CHK003 Is every latency claim split into P50/P95/P99 with a host class named? [Clarity] [Spec §NFR-001 — has P95+P99 on desktop; verify all latency claims do same]
- [ ] CHK004 Are reverse-DNS `app_id` examples shown for every place app-matching is described (`org.kde.kate` not just `kate`)? [Clarity] [Spec §FR-008]
- [ ] CHK005 Does every FR have exactly one normative verb (`MUST` / `MUST NOT` / `SHOULD`), and is `SHOULD` reserved for documented escape hatches? [Clarity] [Gap — FRs currently use "must" inconsistently]

## Requirement Completeness — desktop-integration enumerations (INCOSE C9 Complete sets)

- [ ] CHK006 Are all AT-SPI failure modes enumerated (peer-to-peer bus transition, bus disconnect, `IsEnabled = false` mid-session, app refuses to register, app registers but has no `MENU_BAR`)? [Completeness] [Spec §FR-005; verify against ADR-0024 §Failure modes]
- [ ] CHK007 Is multi-toolkit coverage explicit as a matrix (Qt5 / Qt6 / GTK3 / GTK4 / Electron / Firefox / Tk), each with `supported / partial / out-of-scope` status? [Completeness] [Gap — currently scattered across FR-029 + Out-of-scope + constraints]
- [ ] CHK008 Are all compositor lifecycle events covered (niri restart, `niri msg reload`, niri-IPC schema bump, session resume, focus loss, output hotplug)? [Completeness] [Spec §Scenarios 5–6; verify output-hotplug is mentioned somewhere]
- [ ] CHK009 Is the `DBusMenuHandle` QML_UNCREATABLE constraint cited per ADR (ADR-0007) as the rationale for the bridge owning bus-name acquisition? [Completeness] [Spec §Why; ADR-0007]
- [ ] CHK010 Is every reference app in the v1.0.0 gate (Anki, kate, dolphin, plus 2 GTK apps) named with its specific toolkit version requirement (Qt 6.7+, GTK 4.14+)? [Completeness] [Spec §Constraints]
- [ ] CHK011 Are all `MenuItem` schema fields enumerated with required-or-optional status, value type, and validation rule? [Completeness] [Contracts §active-json-schema]
- [ ] CHK012 Is the GTK4 `GtkPopoverMenuBar` empty-children case explicitly called out as a failure mode (not just a positive-path branch)? [Completeness] [Spec §FR-004; research.md §2]

## Requirement Completeness — supply chain (SLSA + SBOM + attestation)

- [ ] CHK013 Are BOTH CycloneDX 1.7 AND SPDX 2.3 SBOMs specified, attested via `actions/attest-sbom`, and verifiable by `gh attestation verify --predicate-type https://spdx.dev/Document/v2.3`? [Completeness] [Spec §FR-021, §SC-002; contracts/ci-required-checks.md]
- [ ] CHK014 Is the SLSA level declared (L2 minimum, L3 stretch) with rationale tied to the yolo-labz release-engineering standard? [Completeness] [Spec §Why; constitution VI; ~/NixOS/meta/yolo-labz-release-engineering-research.md]
- [ ] CHK015 Is the "no re-tag a release" invariant restated in this spec (not just inherited from the constitution)? [Completeness] [Spec §Out-of-scope or §Constraints]
- [ ] CHK016 Is `SOURCE_DATE_EPOCH = $(git log -1 --format=%ct)` mandated for both the CI archive step and the Nix derivation (FR-019)? [Completeness] [Spec §FR-019, §NFR-005]
- [ ] CHK017 Is reproducibility defined as byte-identical binary across two `nix build` invocations on different hosts (not just same host)? [Clarity] [Spec §NFR-005; verify cross-host scope explicit]

## Requirement Consistency (INCOSE C9 Consistent sets)

- [ ] CHK018 Is the term "active proxy" used identically across spec, plan, contracts (never aliased to "current menu service" or "exported handle")? [Consistency] [Spec §FR-001 vs. ADR-0007]
- [ ] CHK019 Is every FR reachable from at least one ADR (0001–0024), and is the ADR cited inline? [Consistency] [Spec §Functional requirements — verify each FR carries an ADR back-reference]
- [ ] CHK020 Are version-gate thresholds (Sonar coverage 65%, complexity 15, duplication 3%) identical in spec.md and contracts/ci-required-checks.md? [Consistency] [Spec §FR-026 vs. contracts/ci-required-checks.md]
- [ ] CHK021 Does the lane mapping in plan.md §Architecture sketch exactly match the FR ranges in spec.md §Functional requirements? [Consistency] [Plan §Architecture; Spec §FRs]
- [ ] CHK022 Are the 7 user scenarios in spec.md mapped 1-to-1 to user stories in tasks.md (US1–US7)? [Consistency] [Spec §User scenarios; Tasks §User-story → priority mapping]
- [ ] CHK023 Does no requirement silently depend on X11 (e.g., references to `windowId` or `xprop`)? [Consistency] [Spec §FR-004 in spec 001 ruled out windowId — verify spec 004 inherits]

## Acceptance Criteria Quality (Wiegers "Verifiable" + Volere fit criterion)

- [ ] CHK024 Does every SC reference an observable artefact (CI job name, command output, journalctl line, file digest) — not a subjective experience? [Measurability] [Spec §Success criteria]
- [ ] CHK025 Is each SC technology-agnostic enough that a reader without Rust/QML knowledge could verify it? [Clarity] [Spec §Success criteria]
- [ ] CHK026 Does the soak test SC-005 specify the exact metric thresholds (RSS ≤ 50 MB, focus regressions = 0, crash count = 0)? [Measurability] [Spec §SC-005]
- [ ] CHK027 Is the "≤ 10 min user time" budget in SC-004 broken down into per-step time budgets in `quickstart.md`? [Measurability] [Spec §SC-004 vs. quickstart.md]

## Scenario Coverage (INCOSE C10 Complete; Wiegers "Complete")

- [ ] CHK028 Is there a scenario covering "user launches focused app *before* the bridge is up" (boot race)? [Coverage] [Gap — spec 003 FR-014 hints; verify in spec 004]
- [ ] CHK029 Is there a scenario covering "user opens TWO windows of the same app and switches between them" (multi-window same-PID)? [Coverage] [Spec §Scenario 2 → focus apps differ; same-app multi-window may be a gap]
- [ ] CHK030 Is there a scenario covering "AT-SPI menu tree mutates mid-render" (children-changed signal not yet subscribed)? [Coverage] [Spec §FR-006 prerequisites; scenario gap]
- [ ] CHK031 Is there a scenario covering "second monitor hot-plug while a menu is open"? [Coverage] [Gap — multi-monitor is Out-of-scope but hotplug interaction not covered]

## Edge Case Coverage (Wiegers + INCOSE C12 Validatable)

- [ ] CHK032 Are all `MenuItem.label` edge cases addressed (empty, accelerator-marker only `_F`, RTL text, unicode-combining marks)? [Coverage] [Gap]
- [ ] CHK033 Is the stale-AT-SPI-path race documented as a deliberate FR (FR-007) and not as an open question? [Coverage] [Spec §FR-007 — verify it is a closed requirement, not a TODO]
- [ ] CHK034 Are the unsupported toolkits' graceful-degradation paths covered (Electron without `--force-accessibility` → synthetic menu)? [Coverage] [Spec §FR-029 — verify each unsupported case has a documented degraded behaviour]

## Non-Functional Requirements (INCOSE C11 Feasible + Wiegers)

- [ ] CHK035 Is each NFR feasible to verify in CI vs. in manual soak — and is the verification location explicit per NFR? [Measurability] [Spec §NFR-001..NFR-006]
- [ ] CHK036 Is observability (NFR-006) specified with structured log fields, not just prefixes? [Clarity] [Spec §NFR-006]
- [ ] CHK037 Is security (NFR-003) enumerated as a list of systemd hardening flags + an explicit denylist (e.g., "must not exec arbitrary user input")? [Completeness] [Spec §NFR-003]

## Dependencies & Assumptions (Volere snow-card)

- [ ] CHK038 Does every external dependency (Quickshell, noctalia-shell, niri, at-spi2-core, Qt6, GTK4) carry a *single* version constraint (no contradiction between spec.md §Constraints and plan.md §Approach)? [Consistency] [Spec §Constraints vs. Plan §Approach]
- [ ] CHK039 Is every assumption in spec.md §Assumptions tagged with `originator` (who said so) + `rationale` (why) — Volere snow-card style? [Completeness] [Spec §Assumptions]
- [ ] CHK040 Is the self-hosted runner (`vm103`) availability assumption explicit, with a documented graceful-degradation path when offline (FR-023)? [Completeness] [Spec §FR-023; Plan §Risks R5]

## Single-Maintainer & Governance (OpenSSF + bus-factor)

- [ ] CHK041 Is bus-factor = 1 disclosed in SECURITY.md and the rollback path specified (Nix generation pin, bridge `--foreground` fallback)? [Completeness] [Gap — verify SECURITY.md scope after v1.0.0 tag]
- [ ] CHK042 Are the four parallel lanes free of cross-lane cycles (Rust → CI artefact name contract → Nix derivation hash → QML import path)? [Consistency] [Plan §Architecture sketch + §Risks R1–R7]

---

## How to work through this checklist

1. **Sit with the spec open.** Each item references a section by `[Spec §...]` or marks a `[Gap]` — open the cited section and read the surrounding requirement before ticking.
2. **Tick = passes, unticked = open.** Do not partial-tick; a partial pass goes in the `Notes` section below with the specific defect.
3. **`[Gap]` items are spec changes**, not implementation defects — they trigger a `docs(speckit): spec 004 v1.0.1` patch commit on `004-project-completion`.
4. **`[Ambiguity]` items get a 1-sentence clarification** added to the cited section.
5. **`[Conflict]` items need a constitution check** — escalate to `@phsb5321`.
6. **Cap at 3 review iterations** per the project's quality-gate convention; surviving items become explicit open questions in §Open questions of `plan.md`.

## Coverage tally

- INCOSE GtWR v4 characteristics referenced: C1, C2, C4, C9, C10, C11, C12 — full alignment.
- Wiegers SRS checklist items addressed: Correct, Feasible, Necessary, Unambiguous, Verifiable, Complete, Consistent — 7 of 10 (Modifiable + Prioritized + Traceable are inherent to the speckit workflow itself, not testable here).
- SLSA v1.1 L2 / GitHub attestation / SBOM dual-format: covered in CHK013–CHK017.
- Google SRE Launch Coordination Checklist parallels: launch readiness (CHK024–CHK027), capacity / load (CHK035), failure response (CHK006, CHK033), rollback (CHK041).
- Volere quality-gateway gates: Completeness (CHK006–CHK012), Measurability (CHK024–CHK027), Traceability (CHK019, CHK022), Consistency (CHK018–CHK023), Relevancy (implicit), Fit criterion (CHK026).

## Notes (fill during review)

> Reviewer: _______________
> Date: _______________
>
> CHK### — defect summary — proposed fix
> ...

## Open questions surfaced during this drafting

1. Should every FR carry an inline ADR back-reference (CHK019), or is back-reference at the spec-section level sufficient? Currently mixed.
2. Is `[Gap]` CHK028 (boot race) a real coverage hole or already covered transitively by spec 003 FR-014 (zero-paint slot mode when `active.json` absent)? If transitively covered, fold CHK028 into a stricter wording.
3. The "no re-tag" invariant (CHK015) is global per constitution; should it be restated in every spec or treated as inherited?
