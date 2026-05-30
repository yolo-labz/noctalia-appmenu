---
description: "Advance the universal app-menu effort one shippable slice. Preserves AT-SPI-first; continues the fallback → docs → smoke → gtk.Menus → niri → Noctalia → packaging ladder. Reads state, builds, probes, adversarially reviews, updates docs/appmenu-state.md."
---

# /appmenu-forward

The forward-motion loop for noctalia-appmenu. Stops the project stalling in
analysis: pick the next slice on the ladder, implement it, prove it, record
it. Inspired by ADR-0031 (the `.desktop` fallback slice).

## Non-negotiables (read before doing anything)

1. **AT-SPI stays first.** ADR-0024 is the substrate. A fallback only ever
   fires when the AT-SPI walk returns `None`; it must NEVER shadow a real
   `MENU_BAR`. Do not weaken or reorder this.
2. **Do not reintroduce DBusMenu/Registrar as the primary path** (retired
   by ADR-0024 — dead on niri/Qt6).
3. **Honesty.** `source` must tell the truth: `atspi` for native menubars,
   `desktop-fallback` for synthesised fallbacks, `empty` only when no
   identity/menu exists. No faked keystroke/Edit items (the PR #44 trap).
4. **No unsafe `.desktop` Exec.** Never `sh -c`; argv-spawn with field codes
   stripped; re-resolve ids against trusted XDG dirs at click time.
5. **Worktree-first, PR-only.** Never edit the main worktree; never push to
   `main`. See `CLAUDE.md` git recipe.

## Step 0 — orient

- `git rev-parse --show-toplevel` must end in `-NNN-slug` (a worktree). If
  it ends in plain `noctalia-appmenu`, create a worktree first.
- `git status --short`; inspect any local modification before touching it.
- Read: `docs/appmenu-state.md` (most-recent entry = where we are),
  `docs/adr/ADR-0024-atspi-substrate.md`, `docs/adr/ADR-0031-desktop-fallback.md`,
  and the live pipeline: `bridge/src/{proxy.rs,atspi.rs,desktop.rs}`,
  `plugin/BarWidget.qml`. Confirm the menu-source ladder in `proxy.rs`.

## Step 1 — pick the next slice (priority ladder)

Take the highest unfinished item:

1. **`.desktop` fallback** — DONE (ADR-0031). Extend only:
   per-action `icon_name`, locale-aware `Name[xx]`, smarter `app_id`→entry
   resolution for stragglers.
2. **README / docs correction** — keep docs matching reality. Grep for
   stale `pseudo-menu` / `honest-or-hidden` / `collapses to` claims.
3. **Runtime smoke probes** — `cargo run --example desktop_probe -- <app_id>`;
   for a true `active.json` check, run the bridge under a probe
   `publish_service` + temp `XDG_CACHE_HOME` so the production daemon is
   untouched, focus a no-menubar app, read the temp `active.json`.
4. **`org.gtk.Menus` / `GMenuModel` substrate** — real menus for GTK apps
   that export there, slotted ABOVE the desktop fallback, BELOW AT-SPI.
5. **niri window-action enrichment** — move-to-monitor, column ops, etc.
6. **Noctalia UI polish** — distinct styling for `desktop-fallback` vs
   `atspi`; truncation tooltips; popup behaviour.
7. **Packaging / release** — only via `/release-deploy` + `scripts/release.sh`.

Isolate ONE axis per PR (engineering xor aesthetics — never both).

## Step 2 — implement

- Add the smallest change that satisfies the slice. New deps are a last
  resort (deny.toml / osv-scanner / SBOM key off the dep set) — hand-roll
  before adding a crate.
- Tests first where practical: pure parsers/builders get inline unit tests;
  fs/discovery gets a `tempdir` integration test (env-free — inject dirs).

## Step 3 — prove (the quality bar)

```bash
cd bridge
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
git diff --check
```

Runtime: run the relevant probe and capture evidence. State verification
honestly — ✓ verified / ◐ partial / ◯ speculative. Never write ✓ for ◐.

## Step 4 — adversarial review

Invoke `codex:codex-rescue` (or the internal adversarial pass if Codex is
unavailable — say which). Prompt: *be brutal on AT-SPI regression risk,
fake-fallback claims, unsafe `.desktop` Exec, XDG/NixOS path assumptions,
excessive polling, schema breakage for Noctalia, broken `active.json`,
missing tests, `source` mislabelling, niri-IPC hazards, doc overclaims.*
Resolve or document every BLOCKER/MAJOR before merge.

## Step 5 — record

- Prepend a dated entry to `docs/appmenu-state.md`: slice, files, source
  behaviour, tests, smoke evidence, follow-ups, review result, risks.
- Open the PR (worktree-first recipe in `CLAUDE.md`), watch CI, address
  Sonar/Copilot, squash-merge.

## Drift guard

Honour `CLAUDE.md` drift triggers A–I. Two failed iterations on the same
symptom = the architecture is wrong, not the patch — open a redesign spec,
do not ship iteration 3.
