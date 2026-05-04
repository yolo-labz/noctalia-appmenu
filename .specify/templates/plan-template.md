# Implementation plan: <feature title>

**Spec:** `specs/NNN-slug/spec.md`
**Constitution version:** 1.0.0

## Approach

<3-6 paragraphs describing the technical strategy. Reference relevant ADRs.>

## Constitution Check

| Principle | Status | Notes |
|---|---|---|
| I — niri-only v1 | PASS / FAIL / N/A | … |
| II — Sidecar by default | PASS / FAIL / N/A | … |
| III — Worktree-first git | PASS / FAIL / N/A | … |
| IV — Conventional Commits + DCO | PASS / FAIL / N/A | … |
| V — Speckit-driven | PASS / FAIL / N/A | … |
| VI — Release-engineering compliance | PASS / FAIL / N/A | … |
| VII — Graceful degradation | PASS / FAIL / N/A | … |

Any FAIL must be justified inline with a rationale and either an ADR
amendment, a constitution amendment, or an explicit one-off exception
agreed by `@phsb5321`.

## Architecture sketch

```
+------------------+        +------------------+
| component A      | -----> | component B      |
+------------------+        +------------------+
```

## Affected files

- `bridge/src/...`
- `plugin/...`
- `nix/...`
- `tests/...`

## Risks

- **Risk 1** … *Mitigation*: …
- **Risk 2** … *Mitigation*: …

## Rollout

- Dev cycle ends with: `cargo test` green, `nix flake check` green, `qmllint` clean.
- Manual smoke against three apps: kate / qutebrowser / Anki.
- `gh pr merge --squash --delete-branch`.
- Tag v0.X if this completes a planned release.

## Open questions

1. ...
