---
spec: 013-sota-overhaul
purpose: CLAUDE.md-enforceable drift, alignment, escalation rules
case_study: v1.0.5..v1.0.12 popup-dismiss loop (15-16/05/2026)
---

## What drift is

Drift = an agent iterating on the *symptoms* of a bug without re-questioning the *architecture* that produced it. Each fix patches the previous fix's failure mode; commit messages cite version numbers instead of root causes; context fills with intermediate state until the original problem statement is lost. In solo + AI-augmented dev there is no peer reviewer who hasn't read every prior commit — the agent's own iteration history becomes the only check, and confirmation bias turns it into an echo chamber.

Pedro's vault names it a top reliability risk: *"Two modules expressing the same constant by literal value WILL drift"* (`~/Documents/Notes/3. Resources/🖥️ NixOS Configuration/Static Analysis & Testing/2026-05-09 — Methodology — alignment, drift, swarm sanity-checks.md` §1, hereafter `align-methodology`). Same mechanism applies to *commit history*: two consecutive iterations expressing the same fix-intent in different code WILL drift the architecture.

## Drift triggers (observable in git log + transcript)

- **A — Iteration N+1's commit cites iteration N by version.** `align-methodology` §5 — *"PR body cites concrete commit SHAs, not just PR numbers."*
- **B — More than 3 releases in 24 h on one symptom.** `~/.claude/projects/-home-notroot-Documents-Code-yolo-labz-noctalia-appmenu/memory/project_v1_0_6_marathon.md` (7 in 7 h flagged).
- **C — Two consecutive failed smoke tests.** `~/.claude/projects/.../memory/feedback_codex_review_before_iter_3.md` — *"if a single bug has caused ≥ 2 ship attempts to fail, the next step MUST be codex."*
- **D — Human repeats bug report verbatim.** `align-methodology` §2 — "Pedro can name the symptom" real-vs-surface test.
- **E — Iteration reverts to an earlier architecture, no ADR.** Backtrack = unsupervised redesign should have happened earlier. [INFERRED from `feedback_codex_review_before_iter_3.md`.]
- **F — Static check green, runtime red.** `~/.claude/projects/.../memory/feedback_qml_qmllint_not_load_test.md` — *"static accepts what runtime rejects."*
- **G — Deploy claimed, running binary unchanged.** `align-methodology` §3 — three-layer defence (eval / activation / runtime).
- **H — Commit body says "should" / "expected to" not "verified by".** `~/Documents/Notes/2. Areas/wa-memory-investigation-2026-05-09/06-drift-alignment.md` §5 (rule 15: `[x]` requires passing test).

## Decision tree

| Trigger | Action | Entry-point command |
|---|---|---|
| A | Reframe failure as ADR / spec-FR ID before next commit | grep `docs/adr/` + `specs/*/spec.md` for failure-mode name; rewrite subject |
| B | Stop tagging; open redesign spec | `git worktree add ../noctalia-appmenu-NNN-redesign-<bug>`; copy `.specify/templates/spec-template.md` |
| C | Codex adversarial review reading framework source | `codex-rescue --read-source /nix/store/*-quickshell-*/src/ --prompt "be brutal: what still fails in <patch>?"` |
| D | Stop coding; re-read original report + ADRs cold | `git log --grep "<phrase>"`; re-read `docs/adr/` 0001→latest before next edit |
| E | Redesign spec; halt point fixes | Same as B + `gh pr list --search "<bug>" --state all` to map iteration graph |
| F | Add runtime smoke to CI before next release | `qml --offscreen -I plugin/ plugin/BarWidget.qml` in `.github/workflows/ci.yml` |
| G | Verify binary state at runtime before iterating | `systemctl --user restart noctalia-shell.service && journalctl --user -t noctalia-shell --since "1 min ago" \| grep -iE "error\|warn"` |
| H | Block commit; require smoke evidence in body | Pre-commit regex `(should\|expected to\|will probably)` → fail |

## Alignment guardrails

- **Cite the failure mode by ADR / spec-FR ID, not by symptom or prior version.** `06-drift-alignment.md` §5.
- **Read upstream framework source before reverting a fix.** `feedback_codex_review_before_iter_3.md` — codex caught `deleteOnInvisible()==true` by reading `/nix/store/*-quickshell-source/src/window/wlr_layershell.cpp:108-114`.
- **Run runtime smoke (not just lint) before claiming a fix.** `feedback_qml_qmllint_not_load_test.md`.
- **Verify the running binary loaded the new build before iterating.** `feedback_nh_switch_no_shell_restart.md`.
- **Isolate one axis per commit/PR.** `align-methodology` §4 — PR #348 mixed aesthetic + engineering, could not be partially reverted.
- **Swarm for *gathering*, collapse to one parent for *deciding*.** `align-methodology` §1.3 — *"Swarms scale research, not judgement."*
- **Treat 2 failed iterations as evidence the architecture is wrong, not the patch.** `feedback_codex_review_before_iter_3.md`.

## Anti-patterns

- **Forbidden: tagging a release with the bug still observable to the user.** Project CLAUDE.md hard ban #4 extended by case study.
- **Forbidden: writing "this should fix it" without smoke evidence.** `06-drift-alignment.md` §5.
- **Forbidden: iterating after 2 failures without codex or a framework-reading subagent.** `feedback_codex_review_before_iter_3.md`.
- **Forbidden: reverting to an earlier architecture with no ADR documenting why the detour failed.** [INFERRED from v1.0.12 → v1.0.3 backtrack.]
- **Forbidden: trusting `nh os switch` exit code as proof a user-space service loaded new code.** `feedback_nh_switch_no_shell_restart.md`.
- **Forbidden: mixing aesthetic + engineering changes in one commit.** `align-methodology` §6.
- **Forbidden: treating vision-agent or lint-agent severity grades as authoritative.** `align-methodology` §4 (P0 contrast call Pedro-rejected).

## Case study: v1.0.5..v1.0.12

Symptom: popup appmenu does not dismiss on outside click.

| Tag | Commit | Triggers |
|---|---|---|
| v1.0.5 | `36456d4 drop recursive Component` | F, G (4 prior tags never loaded) |
| v1.0.6 | `8802541 skip-list + 30s cache` | — (scope-shift to perf) |
| v1.0.7 | `0be6327 restore Firefox + Chromium menus` | A |
| v1.0.8 | `b68f889 parallel walk` | A |
| v1.0.9 | `b10a312 outside-click dismisses appmenu popup` | D (Pedro re-reported) |
| v1.0.10 | `fdc6d2e popup→Overlay + permanent shield` | A, C |
| v1.0.11 | `77ceb0c shield input via mask Region` | A, C |
| v1.0.12 | `8b4e3f2 xdg_popup grab` | A, C, E (reverts to v1.0.3) |

Where the tree would have intervened:

- After v1.0.10 (trigger C: 2 failed dismiss iterations), the table demands a **codex review reading Quickshell's `PanelWindow::popupGrab` source** — same intervention that resolved the 15/05 freeze (`project_v1_0_6_marathon.md`). Not re-applied. Cost: 2 redundant tags + ~5 h iteration.
- After v1.0.12 (trigger E: architectural backtrack), the table demands a **redesign spec** documenting why v1.0.9..v1.0.11 failed. Absent that ADR, the next agent repeats v1.0.9..v1.0.11. This document is its seed.

Lesson for CLAUDE.md: the guard against drift is not smarter agents — it is **mechanical triggers** on observable signals in git log + transcript. Wire A-H into pre-commit hook and release checklist; agent judgement fails under iteration pressure, a regex does not.
