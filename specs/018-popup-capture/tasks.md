# Tasks

- [x] T001 Reproduce original plugin on a fresh private session: baseline-click/okular.png and first-row-click.json demonstrate dropdown + row action.
- [x] T002 Trace shared popup callers and retain the falsified hypothesis; no runtime patch justified (plan.md).
- [x] T003 Replace CLI-substitution capture with real mouse/OCR/action oracle: capture-check.txt passes both apps; helper contains no bridge action call.
- [x] T004 Capture both apps and replay fresh (replay-check.txt); deliberate miss exits 1 at visible-row gate (negative-check.txt); SHA256SUMS verifies the retained artifacts.
- [x] T005 Publish scoped checks and exact payload head/PR handoff in docs/swarm-2026-09-21.md: aaf3cdcff6845f95e56c4318896daafe040ecade, PR227 OPEN; REST check-runs snapshot records pending CI.
- [ ] T006 Coordinator-only: exact-head different-family gate, inherited CI disposition and merge.
