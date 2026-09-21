# Tasks

- [x] T001 Record exact-head PR/check evidence and repair title metadata.
- [x] T002 Add a red-capable analysis-root regression check, fix configuration,
  and run it plus existing bridge tests.
- [x] T003 Correct release and native/fallback README claims.
- [x] T004 Verify isolated runtime/capture prerequisites and publish either real
  two-app media or an honest blocked capture procedure.
- [x] T005 Prepare reproducibility workflow changes separately with verification.
- [x] T006 Commit canonical report, push scoped PRs, hand review/merge to coordinator.
  PR #225 (config/docs) and #226 (gated workflow) are OPEN; coordinator review
  and merge remain pending, not certified by this worker.

T001–T005 evidence: `docs/swarm-2026-09-21.md`, its linked job/runtime logs,
`cargo-test.txt`, and the separate workflow branch's runnable comparison check.
T004 completes the permitted blocked-procedure alternative, **not** a demo.
T005's full Nix double-build and `cmp` passed (exit 0); identical hash and
commands are recorded in #226's `docs/reproducibility-verification.md`.
