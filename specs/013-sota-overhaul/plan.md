# Implementation plan — spec 013 SOTA overhaul + agent-governance

**Spec:** `specs/013-sota-overhaul/spec.md`
**Branch:** `122-spec-013-plan`
**Author:** @phsb5321
**Date:** 2026-05-20
**Status:** close-out plan (spec 013 funded specs 014 + 015 + PR #94)

## Architecture

Spec 013 was the **research-and-funding phase** triggered after
v1.0.5..v1.0.12 burned 8 plugin tags on one dismiss bug. It scoped
three deliverables:

1. **Research swarm** across Wayland popup dismiss, AT-SPI eager-walk
   strategy, noctalia visual idiom.
2. **Synthesis** into a v1.0.13+ implementation plan covering all four
   open issues at once.
3. **CLAUDE.md governance update** codifying drift detection.

Items 2 + 3 were split across two follow-up specs and one direct
governance PR:

- Plugin redesign for compositor-enforced dismiss → **spec 014**
  (`specs/014-popup-dismiss-redesign`, PR #99 — awaiting Pedro
  architecture pick A-G).
- Bridge routing reliability + visual parity + release-gate harness
  → **spec 015** (`specs/015-ship-ready-completion`, shipped via PRs
  #113, #115–#120).
- Drift-detection table A-H + decision tree → **PR #94** (landed
  directly on CLAUDE.md as governance hardening).

This plan documents disposition of each spec-013 FR and SC, names the
**residue** that neither follow-up spec absorbed, and surfaces the
Pedro decisions needed to close 013 out.

## FR disposition

### Plugin FRs (3.1)

| FR | Title | Disposition | Evidence |
|---|---|---|---|
| FR-001 | Compositor-enforced dismiss (xdg_popup grab) | **MOVED → spec 014** | PR #99 redesign spec; v1.0.12 attempt reverted; Pedro picking from options A-G |
| FR-002 | Eager menu walk on `WindowsChanged`, TTL = PID lifetime | **PARTIAL** | `bridge/src/proxy.rs:412` `MENU_CACHE_TTL` = 30s (not PID-lifetime); no explicit `WindowClosed` eviction wired |
| FR-003 | atspi `ChildrenChanged` / `StateChanged` signal subscription per cached app | **NOT SHIPPED** | Zero matches in `bridge/src/atspi.rs`; gap mitigated by 015's self-heal cascade (FR-006), not the eager-invalidation pattern 013 spec'd |
| FR-004 | Visual treatment binds to shell `Color` + `Style` singletons | **SUBSUMED → spec 015** | 015 FR-004 (`visual-audit.md`, 23 rows, 0 FAIL) + FR-010 (`scripts/verify-tokens.sh` token-discipline grep) |

### Governance FRs (3.2)

| FR | Title | Disposition | Evidence |
|---|---|---|---|
| FR-005 | Drift-detection section in CLAUDE.md | **SHIPPED** | PR #94 added triggers A-H; PR #116 added trigger I |
| FR-006 | Decision tree with concrete entry-points | **SHIPPED** | PR #94 added "Decision tree (trigger → action → entry command)" table |
| FR-007 | Cite agent-drift / alignment material inline | **SHIPPED** | CLAUDE.md "Alignment guardrails (always-on rules)" section |
| FR-008 | Redesign-vs-iterate checklist driven by v1.0.5..v1.0.12 case study | **SHIPPED** | CLAUDE.md "Case study — what NOT to do (v1.0.5..v1.0.12)" section + `specs/015-ship-ready-completion/case-study.md` |

### Research-swarm FRs (3.3)

| FR | Title | Disposition |
|---|---|---|
| FR-009 (Wayland dismiss research) | DONE; output drove spec 014 + PR #93 xdg_popup pivot |
| FR-010 (AT-SPI eager-walk research) | DONE; output drove 015's self-heal cascade + bridge cache TTL |
| FR-011 (Visual idiom research) | DONE; output drove 015's visual-audit + `verify-tokens.sh` |

(Spec 013 itself does not number these as FRs — the research swarm
was scoped as section 3.3. Listed here for completeness.)

## SC disposition

| SC | Title | Disposition |
|---|---|---|
| SC-001 | One-click outside-dismiss with no missed-click logs | **MOVED → spec 014** (blocked on architecture pick) |
| SC-002 | First-focus latency ≤ 16 ms after 5 s preload | **UNMEASURED** — no benchmark harness exists; residue |
| SC-003 | Firefox Bookmarks / Profiles / Tools / Help non-zero children | **MITIGATED** by 015's self-heal cascade (PR #117) but not eager-invalidation per FR-003 |
| SC-004 | Blind side-by-side popup vs Calendar indistinguishable | **SUBSUMED → spec 015 SC-002** (awaiting Pedro signoff) |
| SC-005 | CLAUDE.md decision tree followed on next 2 non-trivial bugs | **IN-FLIGHT** — measured via commit messages over time; v1.0.20–v1.0.23 commits cite triggers as expected |
| SC-006 | Next 90 days ≤ 2 iterations per bug | **NOT YET MEASURABLE** — window starts 2026-05-16, closes 2026-08-14 |

## Residue (work that 013 spec'd but no merged PR shipped)

Three concrete items. Each needs a Pedro disposition decision before
013 can be marked done.

### R1 — FR-002 cache TTL = PID lifetime

**Current:** `MENU_CACHE_TTL` constant in `bridge/src/proxy.rs:412`,
30 s wall-clock. Cache is keyed by service-name (D-Bus connection
name), not PID. No explicit invalidation on niri `WindowClosed`.

**Spec intent:** Cache lifetime = PID lifetime. Eviction triggered by
`niri WindowClosed` for the PID's last window.

**Gap analysis:** 30 s TTL covers "Firefox tab opens, click File, wait
40 s, click again" — the menu re-walks. Spec-013 intent was to skip
the re-walk because cache is still warm. Performance impact: one
extra atspi walk per ≥ 30 s idle. Not user-perceptible in current
benchmarks.

**Recommended disposition:** **DE-SCOPE** — 30 s TTL is a coarser but
working approximation; PID-lifetime eviction is a polish optimization
with no observed user impact. Document as known-deviation, close.

### R2 — FR-003 atspi `ChildrenChanged` / `StateChanged` subscription

**Current:** Bridge calls AT-SPI synchronously per focus event; no
signal subscription on the AT-SPI bus.

**Spec intent:** Bridge subscribes to per-app atspi tree-change
signals; invalidates affected subtree without a focus event. Defeats
Firefox lazy-realisation gap (Bookmarks subtree fills in after first
expand).

**Gap analysis:** 015's self-heal cascade (FR-006, PR #117) catches
the empty-subtree case via *retry on click*, not via *push
invalidation*. User-visible behavior: same end-state (subtree
populates), but with a one-retry latency spike on first expand.

**Recommended disposition:** **DE-SCOPE or FOLLOW-UP SPEC** — self-heal
cascade is the working mitigation. Push-invalidation would eliminate
the retry-latency spike but adds bridge complexity (per-app signal
streams, lifecycle management on `WindowClosed`). Cost/benefit favors
defer unless retry-latency becomes user-perceptible.

### R3 — SC-002 first-focus latency benchmark

**Current:** No measurement harness. v1.0.22 focus-settle bumped 30→150
ms (PR #115) so SC-002's "≤ 16 ms" target is now arithmetically
impossible without re-engineering the focus path.

**Gap analysis:** SC-002's 16 ms target predates the multi-Firefox
routing fix (FR-001 focus-settle). The two pull in opposite directions:
shorter settle → faster popup but routing misroutes; longer settle →
correct routing but slower popup. v1.0.22's 150 ms is the
empirically-validated compromise.

**Recommended disposition:** **REVISE TARGET** — SC-002 needs a new
number aligned with v1.0.22's empirical floor (≥ 150 ms + popup-render
budget). Either rewrite SC-002 to "≤ 250 ms" or de-scope as a
non-functional aspiration superseded by FR-001's routing-correctness
requirement.

## Pedro decisions needed (gating close-out)

1. **R1 — FR-002 PID-lifetime eviction:** DE-SCOPE / FOLLOW-UP / DO?
2. **R2 — FR-003 atspi signals:** DE-SCOPE / FOLLOW-UP-SPEC / DO?
3. **R3 — SC-002 latency target:** REVISE TO 250 MS / DE-SCOPE?

On Pedro's three answers, this plan closes:
- DE-SCOPE picks → write disposition into spec 013 status header, mark
  done, no further PRs.
- FOLLOW-UP-SPEC picks → open `specs/NNN-<title>/` per pick, transfer
  the residue FR/SC text verbatim, mark 013 done with forwarding pointer.
- DO picks → generate `tasks.md` for the chosen items and proceed via
  /speckit:tasks.

## Test strategy

No new code in this plan — disposition-only document. Verification
post-Pedro-decisions:

- For DE-SCOPE picks: confirm spec 013 status header rewrites land and
  no FR/SC remains as "open" in the doc.
- For FOLLOW-UP-SPEC picks: confirm the new spec dir exists with
  spec.md drafted before 013 marks done.
- For DO picks: tasks.md written, then /speckit:implement loop picks
  up from there.

## Risk surface

- **Bias toward DE-SCOPE.** R1, R2 are polish optimizations with
  working mitigations in place. R3 (SC-002 16 ms target) is
  arithmetically wrong post-v1.0.22 — preserving it as "open" creates
  permanent failure in any future doctored gate.
- **No iteration on shipped code.** 013's compositor-dismiss FR (FR-001)
  is owned by spec 014; iterating on dismiss here would duplicate that
  thread and trip drift trigger I.
- **CLAUDE.md trigger I already covers SC-005.** Measuring "decision
  tree followed on next 2 bugs" is happening organically via
  trigger-cite commits (e.g. PRs #112–#117 cite triggers explicitly).
  No additional instrumentation needed.

## Out of plan

- Implementing R1/R2/R3. Plan is disposition-only; implementation (if
  any) happens after Pedro's three picks.
- Modifying CLAUDE.md beyond what spec 015's FR-008 already shipped.
- Closing or re-tagging existing releases — v1.0.13+ has shipped via
  the canonical skill; no rewrite.

## Sequencing

1. **This PR (#122)** — lands `specs/013-sota-overhaul/plan.md` (this
   doc). No code, no behavior change. Reviewable in < 5 min.
2. **Pedro reads R1/R2/R3 disposition asks**, picks one of
   `de-scope` / `follow-up` / `do` for each. Surface via PR comment or
   direct reply.
3. **Follow-up PR per Pedro's picks:**
   - DE-SCOPE-only → single PR rewriting `spec.md` status header to
     `closed` + summary of dispositions. Closes spec 013.
   - Any FOLLOW-UP → open the new spec dir before closing 013.
   - Any DO → run /speckit:tasks on this plan to generate tasks.md
     for the chosen items.

## References

- `specs/013-sota-overhaul/spec.md` — original spec
- `specs/013-sota-overhaul/agent-governance.md` — supersedes most of 3.2
- `specs/013-sota-overhaul/visual-spec.md` — supersedes 3.1 FR-004
- `specs/014-popup-dismiss-redesign/spec.md` — owns 3.1 FR-001 (PR #99)
- `specs/015-ship-ready-completion/spec.md` — owns 3.1 FR-004 + 3.2 FR-008
- PR #94 — drift table A-H + decision tree (governance landing)
- PR #116 — trigger I + HM .backup pre-clear
- PR #117 — self-heal cascade (R2 mitigation)
- PR #115 — focus-settle 30 → 150 ms (R3 mooting)
