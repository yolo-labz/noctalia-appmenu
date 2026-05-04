<!--
Thanks for contributing! Reviewers will look for the following — pre-filling
saves the round-trip. Delete sections that don't apply.
-->

## Summary

<1-3 bullets — what changed and why>

## Spec reference

Spec: `specs/NNN-slug/`. Constitution Check: `specs/NNN-slug/plan.md` §Constitution Check.

## Test plan

- [ ] `cargo test` (bridge)
- [ ] `qmllint plugin/` (plugin)
- [ ] `nix flake check`
- [ ] manual smoke against Anki / kate / qutebrowser
- [ ] integration test added or updated

## ADR impact

- [ ] No ADRs touched
- [ ] ADR-NNNN updated
- [ ] New ADR added at `docs/adr/ADR-NNNN-*.md`

## Release-engineering

- [ ] No new GitHub action introduced
- [ ] Any new action pinned by full 40-char SHA + `# vX.Y.Z` comment
- [ ] `permissions: {}` workflow-level
- [ ] `step-security/harden-runner` step present (release / build jobs)

## Co-author

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```
