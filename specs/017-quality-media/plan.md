# Plan

1. Preserve compact public log excerpts and exact PR heads. No credential changes.
2. Remove the nonexistent Sonar test root; add a stdlib-only root/test-inventory
   regression check and prove red before green. Run the existing bridge tests.
3. Correct README release/status contradictions against release metadata and
   existing ADRs; separately record what was not runtime re-verified.
4. Probe available isolated display/runtime prerequisites without connecting to
   or capturing the daily desktop. Capture only a working actual plugin; otherwise
   publish the procedure and blocker.
5. Prepare the unrelated reproducibility workflow repair separately: Nix's
   `--rebuild` already forces the second derivation build; host `cargo clean` does
   not affect sandbox artifacts. Keep byte comparison and all existing gates.
6. Push scoped feature PRs; deliver exact heads to the coordinator for review.

## Constitution Check

I PASS: niri only. II PASS: no substrate changes. III PASS: exclusive feature
worktrees. IV PASS: conventional commits + DCO. V PASS: spec/plan/tasks precede
implementation. VI PASS: no action pin, provenance or release changes; workflow
slice separately gated. VII PASS: no behavior/fallback changes.

The installed legacy speckit-make engine is not run: it hardcodes retired model
lanes. This uses its spec → plan → tasks → implement → deterministic verification
process; the coordinator supplies the different-family gate.
