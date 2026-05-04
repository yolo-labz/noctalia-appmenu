<!--
Sync Impact Report
- Version: 0.0.0 → 1.0.0 (initial ratification)
- Modified principles: n/a (initial)
- Added sections: Core Principles I-VII, Additional Constraints, Development Workflow, Governance
- Removed sections: n/a
- Templates that depend on this constitution:
  ✅ .specify/templates/spec-template.md (created in same commit)
  ✅ .specify/templates/plan-template.md (created in same commit)
  ✅ .specify/templates/tasks-template.md (created in same commit)
  ✅ CLAUDE.md (cross-references this file)
  ✅ docs/adr/ (decision records consume principles)
- Follow-up TODOs: none
-->

# noctalia-appmenu Constitution

**Version:** 1.0.0
**Ratified:** 2026-05-04
**Last amended:** 2026-05-04

This is the load-bearing rulebook for the project. Every PR's `plan.md` must include a "Constitution Check" section that grades each principle below as PASS / FAIL / N/A and explains FAIL cases. Constitution amendments require a dedicated PR with a Sync Impact Report and a major-version bump on breaking changes.

---

## Core Principles

### I. niri-only in v1; one compositor at a time

**Rule.** v1 targets niri exclusively. Hyprland, Sway, KWin support is out of scope until v2 and only after the focus-tracking abstraction proves itself in production on niri.

**Why.** Each compositor exposes window focus differently. niri's IPC publishes `pid` per window over `event-stream`; Hyprland's `hyprctl` publishes a different schema; KWin uses its own `applicationMenuObjectPath` protocol. Trying to abstract before we have one working implementation produces wrong abstractions.

**How to apply.** New code paths assume niri-IPC is the focus oracle. Refuse "compositor-agnostic" design churn until v2 milestone is open.

### II. Sidecar bridge by default; pure-QML only where it works

**Rule.** Behaviour that requires hosting a D-Bus service, claiming a well-known bus name, or instantiating Quickshell types marked `QML_UNCREATABLE` lives in the Rust sidecar bridge, not in QML.

**Why.** Quickshell exposes `DBusMenuHandle` only via internal use by `SystemTrayItem`. Bus-name acquisition is C++-side. Trying to fake it in QML produces fragile glue that breaks on every Quickshell update.

**How to apply.** When a feature is proposed, ask: "does this need to *publish* on D-Bus, or only *consume*?" Publishing → bridge. Consuming a known fixed-address proxy → QML. The bridge always re-exports a stable proxy when a dynamic address would otherwise force C++ on the QML side.

### III. Worktree-first git workflow

**Rule.** Every feature branch lives in its own `git worktree` directory under `~/Documents/Code/noctalia-appmenu-NNN-slug`. The main worktree at `~/Documents/Code/noctalia-appmenu` stays on `main` forever. `git stash` is forbidden.

**Why.** Two real incidents in the parent NixOS repo (PR #100 v1, PR #141/#142) showed that in-place branching in the main worktree silently inherits stray commits and that activation is path-replacement, not merge. Worktrees eliminate both classes.

**How to apply.** Every contributor follows the recipe in `CONTRIBUTING.md`. CI enforces nothing here — it is discipline + this rule.

### IV. Conventional Commits + DCO sign-off

**Rule.** Every commit subject follows Conventional Commits (`feat(scope): …`, `fix(scope): …`, etc.) under 72 chars. Every commit carries a `Signed-off-by:` trailer (DCO).

**Why.** `git-cliff` generates `CHANGELOG.md` from commits — it must be deterministic. DCO is the lightweight hygiene alternative to a CLA.

**How to apply.** Lefthook + commitlint enforce both at commit time. CI re-validates on PR. Hand-edited `CHANGELOG.md` is a constitution violation.

### V. Speckit-driven feature work

**Rule.** Non-trivial features (any change spanning more than one file in `bridge/src/` or any new public D-Bus surface) require a spec under `specs/NNN-slug/` produced via the `speckit.specify → clarify → plan → tasks` workflow before implementation begins.

**Why.** Spec-first prevents architecture drift. The `plan.md` Constitution Check forces every feature to declare which principles it touches, surfacing conflicts before code lands.

**How to apply.** PR descriptions reference the spec ID. PRs missing a spec for non-trivial changes are blocked. Trivial PRs (typo fixes, dependency bumps) are exempt — judgment call by reviewer.

### VI. yolo-labz release-engineering compliance (non-negotiable)

**Rule.** This repo follows the yolo-labz release-engineering standard verbatim: `actions/attest-build-provenance@v2`, CycloneDX 1.7 + SPDX 2.3 SBOMs, full 40-char SHA action pinning with `# vX.Y.Z` comment trail, Repository Rulesets (not classic branch protection), `permissions: {}` at workflow level, `step-security/harden-runner`, no re-tagging releases, `SOURCE_DATE_EPOCH` everywhere.

**Why.** The standard exists because we audited the alternatives. Deviation requires a documented exception in the relevant ADR, approved by `@phsb5321` (CODEOWNER on the file).

**How to apply.** Every workflow change is reviewed against `~/NixOS/meta/yolo-labz-release-engineering-research.md`. CI runs `actionlint` + `zizmor` on every PR.

### VII. Graceful degradation over feature gating

**Rule.** When the active app has no registered menu, when the registrar daemon is offline, when niri-IPC is unreachable — the widget renders a minimal fallback (app name + Quit / About derived from `.desktop`), or hides itself. It does not error out, log noisily, or crash the bar.

**Why.** Bar widgets are user-facing. A crashed widget breaks the whole bar. A noisy widget is a worse user experience than a quiet pseudo-menu.

**How to apply.** Every code path that talks to D-Bus or niri-IPC has a defined fallback rendered before the failure can propagate. Tests cover the fallback paths.

---

## Additional Constraints

### Stack lock

- **Bridge:** Rust 1.81+, `zbus`, `niri-ipc`, `tokio`, `serde`. No async-std, no smol. No FFI to Qt — talk to D-Bus only.
- **Plugin:** Quickshell ≥ v0.3.0 QML primitives only. No external QML imports beyond `Quickshell.*`, `Qt.*`, `noctalia-shell` exports.
- **Build:** Nix flakes. Direct `cargo` invocation only inside `nix develop` shell or CI runner.

### Versioning

- Semantic Versioning 2.0.0.
- v0.x is alpha; breaking changes allowed in minor bumps.
- v1.0.0 ships when: niri Qt+GTK works on three different apps, integration tests pass on CI runner, README's "Verify the install" recipe works clean on a fresh NixOS box.

### Out of scope (explicitly)

- Firefox / Thunderbird global-menu support (no DBusMenu integration upstream).
- Electron / Chromium support (flag-gated, brittle, not worth the ongoing maintenance).
- Multi-monitor menubar duplication (v1 = focused-output only; multi-output deferred).
- Alt-letter mnemonics + global Alt-F intercept (deferred to v2 — no clean Quickshell hook in v1).
- Hosting `com.canonical.AppMenu.Registrar` ourselves (we delegate to `vala-panel-appmenu`'s daemon).

---

## Development Workflow

```
constitution (this file, immutable until amended via PR)
    └── speckit.specify ──→ specs/NNN-slug/spec.md
            └── speckit.clarify (only if [NEEDS CLARIFICATION] tags exist)
                    └── speckit.plan ──→ specs/NNN-slug/plan.md (Constitution Check)
                            └── speckit.tasks ──→ specs/NNN-slug/tasks.md (≤25 items)
                                    └── speckit.implement (≤25 deliverable PRs)
                                            └── speckit.analyze (post-merge cross-artifact lint)
```

Specs cannot exceed 25 tasks. Bigger features split into 100-, 110-, 120-numbered slugs.

---

## Governance

- **Amendment process.** Open a PR titled `governance(constitution): vX.Y.Z — short summary`. Bump version per the rules below. Include a Sync Impact Report comment at the top of `constitution.md`. Update every dependent template referenced in the report. Squash-merge after `@phsb5321` approval (CODEOWNER).

- **Versioning policy** (this file's version, not the project's):
  - **MAJOR** — backward-incompatible removal/redefinition of a principle.
  - **MINOR** — new principle added, or material expansion of an existing one.
  - **PATCH** — wording fixes, typo corrections, non-semantic clarifications.

- **Compliance review.** `speckit.analyze` runs in CI on every PR and reports principle drift. Quarterly manual audit by `@phsb5321` against the live state of the codebase.

- **Conflict resolution.** When this constitution conflicts with the user's global rules at `~/.claude/CLAUDE.md` or with the parent NixOS `CLAUDE.md` workflow rules: this constitution wins for project-specific decisions; the global rules win for cross-project hygiene (DCO, Conventional Commits, worktree-first). Document conflicts in an ADR.
