# 018 — Legacy topbar popup evidence

Date: 21/09/2026. Scope: R5c, branch 227-popup-capture only.

## Requirements

- FR-001: In an owned, fresh private desktop, click the actual topbar menu,
  observe its dropdown, click a visible leaf row and observe the corresponding
  application action for both empty-profile Okular and Dolphin.
- FR-002: Preserve reproducible source, native-menu/PID receipts, real pixels
  and interaction records. No bridge CLI substitute, app-menubar substitute,
  mocked rendering, keyboard action or daily desktop capture.
- FR-003: Change runtime code only for a cause demonstrated in the private
  runtime, with a red/green regression. Do not patch a falsified hypothesis.
- FR-004: No claim about the current Rust Noctalia host; no deploy, merge,
  workflow/CODEOWNERS change or sibling-worktree mutation.

## Observation

Hypothesis: the legacy AppMenu popup implementation prevents a populated Help
menu from becoming visible. **Falsified** at base 29d0995: a fresh private
session, original plugin, topbar mouse click (397,15) renders the dropdown;
mouse click (455,247) creates a real About Okular window with the app's PID.
Baseline screenshots and first-row-click.json are retained. The prior R5b
invisibility is not reproduced and its historical cause remains unproven.

A separate harness failure is demonstrated: nesting the runtime under the long
Pi scratch path exceeds Unix SUN_LEN and niri does not create NIRI_SOCKET.
Fresh short /tmp/r5c-* roots restore private IPC; no global fallback is allowed.
