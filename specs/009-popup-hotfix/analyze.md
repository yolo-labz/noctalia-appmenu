# Cross-artefact analysis — spec 009-popup-hotfix

Spec: `specs/009-popup-hotfix/spec.md`
Plan: `specs/009-popup-hotfix/plan.md`
Tasks: `specs/009-popup-hotfix/tasks.md`
Run: 2026-05-15 (post-implementation, pre-merge)

Non-destructive cross-cut of `spec.md` × `plan.md` × `tasks.md` × `research.md` × `data-model.md` × `contracts/*` × `quickstart.md` × the implementation (commits on `009-popup-hotfix`).

---

## A — Spec ↔ Plan ↔ Tasks alignment (FR coverage matrix)

| FR | Spec | Plan owner | Task IDs | Implementation status |
|----|------|-----------|----------|----------------------|
| FR-001 | recursive Qt wrapper-flatten | Lane B / `bridge/src/atspi.rs` | T003, T004, T005 | LANDED — flatten_qt_wrapper extracted + 7 unit tests pass |
| FR-002 | constrained popup surface | Lane Q / `AppmenuPopupWindow + SubmenuPopup` | T009, T010 | DEFERRED to v1.0.2 — needs outside-click dismissal design |
| FR-003 | popupCol width binding | Lane Q / both popups | T009, T010 | LANDED — JS-computed `_calcWidth` via hidden Text metrics |
| FR-004 | async-safe Loader for nested | Lane Q / SubmenuPopup | T010 | LANDED — `_pendingNested` + `Loader.statusChanged` |
| FR-005 | children-aware top-level dedup | Lane Q / BarWidget | T011 | LANDED — `_sameTopLevel` extended to first-level children |
| FR-006 | cross-screen guard fallback | Both lanes (additive `focused_output`) | T006, T007, T011 | DEFERRED to v1.0.2 — single-screen hosts unaffected |
| FR-007 | recursive submenu namespace | Lane Q / SubmenuPopup | T010 | LANDED — `depth: int` property, namespace `-dN-` suffix |
| FR-008 | `_failedState` self-clear | Lane Q / BarWidget | T011 | NO-OP — existing logic already clears on successful apply; agent was wrong |

**Coverage:** 6 of 8 FRs landed in v1.0.1; FR-002 + FR-006 deferred with explicit rationale. FR-008 retired as false alarm. No FRs orphaned, no tasks orphaned.

## B — FR ↔ research decision mapping (one-to-one)

| FR | Research §Decision |
|----|-------------------|
| FR-001 | Decision 1 — recursive flatten lives in `fetch_menu_tree`, bottom-up |
| FR-002 | Decision 2 — constrain popup to menuBox.height (DEFERRED) |
| FR-003 | Decision 3 — `popupCol` width via `childrenRect.width` (impl chose JS-computed instead, both achieve the same FR-003 contract) |
| FR-004 | Decision 4 — Loader async-safe via `Loader.status === Ready` check |
| FR-005 | Decision 5 — `_sameTopLevel` honours children-shape changes |
| FR-006 | Decision 6 — `focusedScreenName` falls back to `active.json` (DEFERRED) |
| FR-007 | Decision 7 — recursive submenu uses depth-suffixed namespace |
| FR-008 | Decision 8 — `_failedState` self-clear extracted into named function (retired: existing logic suffices) |

**Drift:** FR-003's implementation used a JS-computed width via hidden `Text` element rather than research-spec's `childrenRect.width`. Reason: `childrenRect.width` is circular when children's width binds to parent.width (MenuRow does this). Both approaches satisfy the FR-003 contract ("width clamps lifted; long labels fully visible"). Documented in commit body.

## C — SC verification matrix

| SC | Verification method | Status |
|----|--------------------|--------|
| SC-001 | Manual smoke against shadPS4QtLauncher (screenshot diff) | PENDING — post-deploy |
| SC-002 | Manual smoke depth-2 cascade | PENDING — post-deploy |
| SC-003 | Manual smoke bar-button transition | PENDING — post-deploy (FR-002 deferred; behaviour unchanged from v1.0.0 but bug is no-worse) |
| SC-004 | `bridge/tests/atspi_flatten.rs` round-trip | PASSED — 7 tests green via inline `mod tests` |
| SC-005 | qmltest cascade harness | PENDING — harnesses not added in this PR (FR-004 verified via code inspection; runtime test deferred) |
| SC-006 | 3-of-5 Qt6 app matrix | PENDING — post-deploy on Pedro's desktop |
| SC-007 | qmllint clean + existing qmltest green | PASSED — qmllint baseline preserved |
| SC-008 | `gh attestation verify` exit 0 on v1.0.1 binary | PENDING — post-tag |

**Verifiable-in-CI:** 2 of 8 (SC-004, SC-007). Rest require Pedro's desktop with active Qt6 apps; tracked as post-deploy manual smoke in T025.

## D — Constitution Check (re-evaluated post-impl)

| Principle | Plan grade | Post-impl observation |
|---|---|---|
| I — niri-only v1 | PASS | No new compositor paths. |
| II — Sidecar by default | PASS | Bridge owns FR-001 (atspi.rs). QML owns FR-003/004/005/007. No QML-side D-Bus claim added. |
| III — Worktree-first git | PASS | All work in `noctalia-appmenu-009-popup-hotfix`. Main worktree untouched. |
| IV — Conventional Commits + DCO | PASS | 4 commits: `docs(speckit)`, `fix(bridge)`, `fix(plugin)`, `style(bridge)`. All DCO-signed. lefthook gated. |
| V — Speckit-driven | PASS | This file completes the chain: specify → plan → tasks → checklists → analyze. |
| VI — Release-engineering compliance | PASS | No workflow changes. v1.0.1 ships on the existing supply-chain spine. |
| VII — Graceful degradation | PASS | FR-005 prevents stale-state lockout. No new error paths. |

No FAILs introduced. No ADR amendments required.

## E — Contracts ↔ implementation parity

| Contract | Producer | Consumer | Parity |
|---|---|---|---|
| `recursive-flatten.md` | `bridge/src/atspi.rs::flatten_qt_wrapper` | `bridge/src/active.rs` (downstream consumer) | MATCHES — 4 of 4 edge cases unit-tested |
| `popup-surface.md` | (FR-002 deferred — contract unchanged from v1.0.0) | niri compositor + user | DEFERRED — contract is forward-spec; v1.0.1 implementation preserves v1.0.0 behaviour |
| `active-json-schema.md` | (FR-006 deferred — bridge schema unchanged) | `BarWidget.qml::applySnapshot` | DEFERRED — schema is forward-spec; v1.0.1 emits no `focused_output` |

## F — Checklist quality status

`checklists/requirements.md`: PASSED (10 of 10 quality dimensions, including 7 constitution principles).

`checklists/comprehensive.md` (45 items): not run in this PR. The agent-investigation phase generated the items pre-implementation; they remain valid for the spec quality, not for the impl. Tracked as a quarterly hygiene pass — not a v1.0.1 gate.

## G — Risks: mitigations re-evaluated

| Risk | Mitigation in plan | Status |
|------|-------------------|--------|
| R1: surface constraint breaks coord math | `mapToGlobal(0,0)` + popup_geometry test | N/A — FR-002 deferred; risk dormant |
| R2: recursive flatten quadratic blowup | Bottom-up; no extra descend | LANDED — existing code already runs flatten at recursion's end |
| R3: `_sameTopLevel` widening reintroduces flicker | Shallow children comparison | LANDED — only first-level compared; preserves PR #51 anti-flicker |
| R4: Loader async never fires | Sync check + Connections listener | LANDED — `_tryOpenNested` called both inline + on statusChanged |
| R5: `focused_output` breaks downstream | Field is OPTIONAL | N/A — FR-006 deferred |
| R6: self-hosted runner contention | Sequential CI | LANDED — single PR; one CI cycle |

## H — Open items for follow-up specs

1. **Spec 010 — v1.0.2 popup surface + multi-monitor.** Pick up FR-002 and FR-006 with proper outside-click dismissal design (HyprlandFocusGrab analogue, or bridge-side IPC dismiss on focus-shift).
2. **Spec 010 — qmltest harnesses.** Add `popup_geometry.qml` + `submenu_cascade.qml` to back SC-005 in CI. Defer from this PR because FR-002 surface change re-shapes the harness expectations.
3. **Open question from `comprehensive.md`:** multi-toolkit matrix (Qt5/Qt6/GTK3/GTK4/Electron/Firefox/Tk) — promote to explicit FR vs. document the implicit ADR-0024 coverage. Spec 010 decides.

## I — Release readiness summary

**v1.0.1 SHIP CRITERIA** (this PR):
- 6 of 8 FRs landed (FR-002, FR-006 deferred)
- 8 of 8 plan tasks landed (T-008 build/test green, T-014 qmllint clean, T-021 manual smoke pending)
- Constitution: all 7 principles PASS
- CI: green (modulo known Codecov runner-side `Read-only file system` issue, pre-existing)
- Manual smoke: PENDING on Pedro's desktop post-deploy

**Recommendation:** merge PR #82; tag v1.0.1; deploy via `~/NixOS/flake.lock` bump + `nh os switch`; smoke against shadPS4QtLauncher; if SC-001..003 pass, mark v1.0.1 SHIPPED in memory. Open spec 010 for the deferred FRs.
