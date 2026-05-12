# Cross-artifact analysis — 004-project-completion

**Generated:** 2026-05-12
**Inputs:** `spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `contracts/*`, `quickstart.md`, `checklists/{requirements,release-readiness}.md`
**Constitution version:** 1.0.0
**Mode:** static cross-artifact lint, no implementation read

This document records what `/speckit:analyze` surfaced after the spec→plan→tasks→checklist chain landed (commits `b0990fa`..`5ac1014`). Findings are split into "patched-in-this-commit" (mechanical, low-risk fixes applied alongside this report) and "tracked-as-open" (require Pedro's scope call).

---

## 1. Patched in this commit

### A1 — ADR back-references missing per FR cluster (CHK019)

**Symptom.** `spec.md §Functional requirements` groups FRs by lane but most clusters carry no inline ADR citation. Plan.md §Approach lists 9 ADRs (0011, 0013, 0015, 0017–0019, 0024); spec.md cites ADR-0024 once (in §Why), ADR-0008 once (in FR-010), and ADR-0007 only via spec 002's superseded text.
**Impact.** A reviewer reading a single FR cannot trace it back to the decision record without grepping. Violates checklist item CHK019 + constitution principle V's intent ("spec-first prevents architecture drift").
**Patch.** Added one-line `**ADR refs:** ...` block at the head of each FR cluster in `spec.md`, citing the load-bearing ADRs:

| FR cluster | ADR refs added |
|---|---|
| FR-001..FR-003 (focus tracker) | ADR-0009 (debounce policy), ADR-0016 (event-stream schema) |
| FR-004..FR-009 (AT-SPI walker) | ADR-0024 (AT-SPI substrate), ADR-0022/ADR-0023 (superseded — link only) |
| FR-010..FR-013 (plugin) | ADR-0008 (popup window for submenus), ADR-0018 (bar widget API), ADR-0019 (always-visible widget), ADR-0020 (fixed-width slot) |
| FR-014..FR-020 (Nix) | ADR-0011 (HM module), ADR-0024 (env-var rationale) |
| FR-021..FR-025 (CI / release) | ADR-0012 (self-hosted runner), ADR-0013 (runner-agnostic), ADR-0014 (local-first CI) |

### A2 — "No re-tag a release" invariant not restated in spec.md (CHK015)

**Symptom.** Constitution principle VI + Outscope list bans re-tagging; `plan.md §Rollout` mentions it; `spec.md` does not restate.
**Impact.** A maintainer who reads spec.md alone could assume re-tagging is acceptable for "small" post-tag defects. CHK015 explicitly fails.
**Patch.** Added a bullet to `spec.md §Constraints / dependencies` restating the invariant inline.

### A3 — Quickstart Quickshell-version drift (CHK038)

**Symptom.** `spec.md §Constraints` says `Quickshell ≥ 0.3.0 (no upper bound)`; `quickstart.md §0` says `quickshell 0.3.x` (restricts to 0.3.x).
**Impact.** Drift between spec scope and verification recipe.
**Patch.** Reworded `quickstart.md §0` check to "quickshell 0.3.0 or newer".

### A4 — Output hotplug not explicit in Out-of-scope (CHK031)

**Symptom.** Multi-monitor menubar duplication is out-of-scope. Output hotplug *while a popup is open* is a distinct edge case not addressed.
**Impact.** Reviewer cannot tell whether hotplug → popup-on-vanished-screen is a known limitation or an undiscovered bug.
**Patch.** Added an Out-of-scope bullet documenting hotplug as a v2 follow-up.

---

## 2. Tracked as open questions (not patched)

These need a scope decision before patching. Captured in `plan.md §Open questions` (item 3+ to be added) for the spec PR review.

### O1 — Boot-race scenario (CHK028)

**Question.** Should we add a Scenario 8 covering "user launches focused Qt6 app before the bridge user-unit comes up" (boot race)?
**Current state.** Spec 003 FR-014/FR-015 cover the *plugin's* behaviour when the file/IPC target is missing; spec 004 does not have a matching scenario for the *bridge's* late-start coverage. The bridge's systemd unit is `WantedBy=graphical-session.target` — race is bounded but not zero.
**Recommendation.** Fold the implicit coverage into Scenario 5 (AT-SPI bus crash) with a new "or the bridge unit comes up later than the focused app" clause. Defer to PR review for the wording.

### O2 — Multi-window same-PID (CHK029)

**Question.** Two qutebrowser / kate windows on the same PID with different menu trees (recent files differ) — covered by Scenario 2 (focus moves between *different apps*) or a separate scenario?
**Current state.** Spec 002 (superseded by ADR-0024) had Scenario 4 for this. Spec 004 inherits the concern via FR-005 + the AT-SPI walker's per-window subtree fetch, but no scenario explicitly tests the same-PID case.
**Recommendation.** Same-PID multi-window is *the same scenario* under AT-SPI substrate (the walker re-fetches per focus event regardless of PID identity). Document this in `research.md §2` as a non-issue, no scenario addition needed. Verify via Lane A's child spec.

### O3 — Mid-render AT-SPI mutation (CHK030)

**Question.** App rebuilds its widget tree while a popup is open → render uses stale paths.
**Current state.** FR-007 (click re-fetch) addresses the click-time race; it does not address the render-time race.
**Recommendation.** Out of scope for v1.0.0 — Eager re-walk on `children-changed` is FR-006's prerequisite groundwork; the actual subscription is post-v1. Document explicitly in §Out-of-scope.

### O4 — RTL / unicode-combining marks in `MenuItem.label` (CHK032)

**Question.** Anki / kate users with RTL locales or apps with combining marks in menu labels (e.g. accents) — covered?
**Current state.** No explicit coverage. AT-SPI accessibles carry already-translated label text; the bridge passes through unchanged. RTL rendering depends on the QML row delegate's `LayoutMirroring` posture, which is not specified.
**Recommendation.** Add to FR-010..FR-013 (Lane B child spec) — Lane B verifies row delegate handles `LayoutMirroring.enabled = (Qt.application.layoutDirection === Qt.RightToLeft)` correctly. Cheap to test, valuable to ship.

### O5 — `[Gap]` SECURITY.md bus-factor disclosure (CHK041)

**Question.** Per OpenSSF maintainer evaluation guide + bus-factor=1 reality. Should v1.0.0 include a `SECURITY.md` update?
**Current state.** Existing `SECURITY.md` points users at `/security/advisories/new` (per constitution + yolo-labz release-engineering standard). Bus-factor disclosure is not present.
**Recommendation.** Add a one-paragraph "Maintainer status" section to `SECURITY.md` as part of Lane D's docs work. Lane D child spec absorbs.

---

## 3. Consistency tally

| Checklist item | Status | Notes |
|---|---|---|
| CHK018 (term aliasing — "active proxy") | PASS | `plan.md §Approach` uses "fixed proxy" once; verified it's the *same* concept referenced by ADR-0007, semantically identical, not a defect. |
| CHK019 (ADR back-refs) | PATCHED (A1) | All five FR clusters now carry ADR refs inline. |
| CHK020 (Sonar threshold consistency) | PASS | spec.md FR-026 + contracts/ci-required-checks.md table agree exactly. |
| CHK021 (FR↔Lane mapping) | PASS | plan §Affected files matches spec FR clusters 1-to-1. |
| CHK022 (US↔Scenario 1-to-1) | PASS | tasks.md §User-story mapping is a clean bijection with spec §Scenarios. |
| CHK023 (no silent X11 dependency) | PASS | spec 001 FR-004 explicitly rules out `windowId` (X11 XID); spec 004 inherits transitively. No new X11 references. |
| CHK015 (no re-tag invariant) | PATCHED (A2) | Now restated in spec.md §Constraints. |
| CHK038 (version constraint consistency) | PATCHED (A3) | Quickstart wording aligned with spec.md. |
| CHK031 (output hotplug) | PATCHED (A4) | Now explicit in §Out-of-scope. |

Six items pass without change; four required mechanical patches; five remain open as scope questions.

---

## 4. Constitution check (re-run post-patches)

| Principle | Status | Delta from plan.md |
|---|---|---|
| I — niri-only v1 | PASS | unchanged |
| II — Sidecar by default | PASS | unchanged |
| III — Worktree-first git | PASS | unchanged |
| IV — Conventional Commits + DCO | PASS | unchanged |
| V — Speckit-driven | PASS | strengthened — analyze.md is itself a speckit artefact |
| VI — Release-engineering compliance | PASS | unchanged |
| VII — Graceful degradation | PASS | unchanged |

No regressions. All gates green.

---

## 5. Ready to advance?

After committing the patches (A1–A4) onto `004-project-completion` and resolving the five open questions (O1–O5) in PR review:

- `/speckit:implement` can fire the 4 parallel claude-code worker shells per plan.md §Approach.
- Child sub-specs `005`/`006`/`007`/`008` each go through their own speckit chain inside the worker session.
- Parent reviews each child PR before merge.

Lane A (Rust) starts first per `tasks.md §Phase 3` MVP gate; Lanes B/C dispatch in parallel after Lane A merge; Lane D last.
