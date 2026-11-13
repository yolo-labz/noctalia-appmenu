# Quality configuration and honest demo evidence

Date: 21/09/2026. Generator: OpenAI GPT-6 Astra/xhigh.

- FR-001: All configured analysis roots exist; keep every current Rust test and
  source in scope, with no new exclusions or relaxed thresholds.
- FR-002: Diagnose dependency PRs #219, #222, #223 and #224 from exact-head jobs;
  distinguish stale failures, runner failures, configuration failures and gating.
- FR-003: README describes the released version and distinguishes native menus
  from fallback behavior. Do not present historical toolkit observations as a
  new runtime verification.
- FR-004: A public demo requires the actual QML plugin, bridge and two supported
  apps in a disposable isolated display/session. No daily-session changes or
  capture. If blocked, record the exact gate and a reproducible capture recipe.
- FR-005: Keep workflow changes in a separate PR. Coordinator owns independent
  GLM-5.3 review and merge; no worker deploy, merge or self-certified gate.
