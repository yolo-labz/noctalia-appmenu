<!--
Sync Impact Report
- Version: 1.1.0 → 1.1.1 (PATCH: wording correction — stale registrar reference in Principle VII)
- Modified principles:
  - VII. Graceful degradation — replaced "when the registrar daemon is offline" with
    "when the AT-SPI bus is unreachable or the app is not on the a11y registry"
    to match the AT-SPI substrate (ADR-0024). v1.0.0+ has no registrar daemon.
- Modified sections: n/a (text-only fix inside Principle VII rule)
- Added sections: n/a
- Removed sections: n/a
- Templates that depend on this constitution: unchanged
- Follow-up TODOs: none
-->

<!--
Prior Sync Impact Report (v1.0.0 → 1.1.0, 2026-05-25):
- Modified principles:
  - VI. yolo-labz release-engineering — SBOM version (CycloneDX 1.7 → 1.6 per ADR-0026),
    attest-build-provenance pin (v2 → v4.1.0 per fleet rollout standard)
- Modified sections:
  - "Additional Constraints" / Stack lock — Rust 1.81+ → Rust 1.95+ (match flake.lock toolchain)
  - "Additional Constraints" / Versioning — v1.0.0 shipped 13/05/2026 (was target); v1.0.25 current
  - "Out of scope" — Removed three stale items: (1) Firefox/Thunderbird DBusMenu claim
    superseded by AT-SPI partial support (ADR-0024); (2) Electron/Chromium hard-out claim
    contradicted by README's `--force-accessibility` workflow; (3) Registrar-delegation
    line — bridge no longer talks to com.canonical.AppMenu.Registrar (ADR-0024).
  - "Additional Constraints" / Substrate — NEW subsection naming AT-SPI as the v1 substrate,
    pointing at ADR-0024.
- Added sections: Substrate (under Additional Constraints).
-->

# noctalia-appmenu Constitution

**Version:** 1.1.1
**Ratified:** 2026-05-04
**Last amended:** 2026-05-26

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

**Rule.** This repo follows the yolo-labz release-engineering standard verbatim: `actions/attest-build-provenance@v4.1.0` (current fleet pin, full 40-char SHA), CycloneDX 1.6 + SPDX 2.3 SBOMs (1.6 not 1.7 — `syft` pre-1.34 emits 1.6; see [ADR-0026](../../docs/adr/ADR-0026-cyclonedx-1.6-syft-constraint.md)), full 40-char SHA action pinning with `# vX.Y.Z` comment trail, Repository Rulesets (not classic branch protection), `permissions: {}` at workflow level, `step-security/harden-runner`, no re-tagging releases, `SOURCE_DATE_EPOCH` everywhere.

**Why.** The standard exists because we audited the alternatives. Deviation requires a documented exception in the relevant ADR, approved by `@phsb5321` (CODEOWNER on the file).

**How to apply.** Every workflow change is reviewed against `~/NixOS/meta/yolo-labz-release-engineering-research.md`. CI runs `actionlint` + `zizmor` on every PR.

### VII. Graceful degradation over feature gating

**Rule.** When the active app has no exported menu, when the AT-SPI bus is unreachable or the app is not on the a11y registry, when niri-IPC is unreachable — the widget renders a minimal fallback (app name + Quit / About derived from `.desktop`), or hides itself. It does not error out, log noisily, or crash the bar.

**Why.** Bar widgets are user-facing. A crashed widget breaks the whole bar. A noisy widget is a worse user experience than a quiet pseudo-menu.

**How to apply.** Every code path that talks to D-Bus or niri-IPC has a defined fallback rendered before the failure can propagate. Tests cover the fallback paths.

---

## Additional Constraints

### Stack lock

- **Bridge:** Rust 1.95+ (toolchain pinned via `rust-overlay` in `flake.lock`; bumps land via Dependabot/flake-update PRs). `zbus`, `niri-ipc`, `tokio`, `serde`. No async-std, no smol. No FFI to Qt — talk to D-Bus / AT-SPI only.
- **Plugin:** Quickshell ≥ v0.3.0 QML primitives only. No external QML imports beyond `Quickshell.*`, `Qt.*`, `noctalia-shell` exports.
- **Build:** Nix flakes. Direct `cargo` invocation only inside `nix develop` shell or CI runner.

### Substrate

- **AT-SPI menubar walker** is the v1 substrate ([ADR-0024](../../docs/adr/ADR-0024-atspi-substrate.md)). The bridge connects to the a11y bus via `org.a11y.Bus.GetAddress()`, walks `org.a11y.atspi.Registry` root children, PID-matches against niri's `WindowFocusChanged.pid`, and exports the walked menu tree at `org.noctalia.AppMenu /org/noctalia/AppMenu/Active`.
- The pre-v1 `com.canonical.AppMenu.Registrar` / DBusMenu pipeline is preserved in [`docs/architecture/dbusmenu.md`](../../docs/architecture/dbusmenu.md) for historical context. The bridge no longer talks to the registrar daemon.

### Versioning

- Semantic Versioning 2.0.0.
- v0.x was alpha (breaking changes allowed in minor bumps).
- v1.0.0 shipped 2026-05-13 against the original gate (niri Qt+GTK on three apps, integration tests on CI, README's "Verify the install" recipe). Subsequent v1.0.x patches address focus-resolution, popup-dismiss, and supply-chain follow-ups.

### Out of scope (explicitly)

- **Chrome's hamburger menu.** AT-SPI shape does not expose a `MENU_BAR` role; the bridge's learned-skip caches the no-walk outcome per [ADR-0029](../../docs/adr/ADR-0029-learned-no-menubar-skip.md). Other Chromium-based apps work via `--force-accessibility`.
- Multi-monitor menubar duplication (v1 = focused-output only; multi-output deferred).
- Alt-letter mnemonics + global Alt-F intercept (deferred to v2 — no clean Quickshell hook in v1, see [ADR-0010](../../docs/adr/ADR-0010-no-keybind-intercept-v1.md)).
- Accelerator dispatch (deferred — [ADR-0028](../../docs/adr/ADR-0028-fr-003-accelerator-deferred.md); niri-ipc 26.4.0 gap).

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
