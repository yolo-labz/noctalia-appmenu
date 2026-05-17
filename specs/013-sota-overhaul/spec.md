---
spec: 013-sota-overhaul
title: noctalia-appmenu state-of-the-art overhaul + agent-governance
status: draft
created: 2026-05-16
owner: pedro
---

# 013 — State-of-the-art overhaul + agent-governance

## 1. Why this spec exists

Pedro field report 16/05/2026 21:00 BRT: after seven plugin releases
(v1.0.5..v1.0.11) the appmenu **still** does not dismiss on outside
click, the menu **still** isn't instant on first focus, the lazy-realised
Firefox subtrees still surface as empty, and the visual treatment lags
the rest of noctalia-shell (image #4, image #5). The iterations have
been individually small but collectively a drift away from a coherent
design — each hotfix was scoped to "the specific bug Pedro reported",
not the underlying root cause.

This spec funds:

1. A **research swarm** across the three problem domains (Wayland popup
   dismiss, AT-SPI eager-walk strategy, noctalia visual idiom) to
   establish state-of-the-art patterns rather than continuing the
   incremental hotfix loop.
2. A **synthesis pass** that converts the research into a coherent
   v1.0.13+ implementation plan covering all four open issues at once.
3. A **CLAUDE.md governance update** that codifies the lessons of the
   v1.0.5..v1.0.12 drift — decision trees for "when to keep iterating
   vs. when to redesign", explicit drift-detection signals, alignment
   guardrails based on the `~/Documents/Notes` agent-drift / alignment
   material.

The goal is to never spend seven minor releases on a single dismiss
bug again.

## 2. User scenarios

### 2.1 Pedro opens any app menu

- **Given** Pedro has used noctalia for ≥ 30 seconds since shell start
  (every visible window's menu has had time to pre-walk),
- **When** Pedro clicks any top-level menu button on the bar,
- **Then** the popup appears within one frame (≤ 16 ms perceived latency)
  with the **complete** submenu tree visible — including Firefox's
  lazily-realised Bookmarks / Profiles / Tools / Help.

### 2.2 Pedro dismisses any app menu

- **Given** any popup or cascade of submenus is open,
- **When** Pedro clicks anywhere outside the popup chain (Firefox content,
  a different bar widget, a different monitor, the desktop), **or**
  presses `Esc`, **or** alt-tabs to a different application,
- **Then** the entire chain collapses immediately (compositor-enforced)
  with no leftover surfaces.

### 2.3 Pedro reviews the visual

- **Given** the menu is open,
- **When** Pedro compares it side-by-side to the noctalia Calendar
  (image #4) or Control Center (image #5),
- **Then** the surface uses the same radius/border/typography/colour
  vocabulary (`Color.mSurface`, `Style.radiusL`, `Color.mPrimary`
  accents on hover) — the menu reads as "part of noctalia" rather than
  a third-party graft.

### 2.4 A future Claude session avoids the drift trap

- **Given** a future agent (or me, in a later session) is debugging the
  appmenu (or any similarly fiddly subsystem),
- **When** the agent has produced two failed iterations on the same bug
  in a row,
- **Then** the project CLAUDE.md decision tree triggers a hard pivot —
  spawn a codex review, spawn parallel research subagents, or
  redesign — *instead of* drafting v1.0.NNN+3 with the same architecture.

## 3. Functional requirements

### 3.1 Plugin (appmenu) — v1.0.13+

- **FR-001** — Popup dismiss must be compositor-enforced (xdg_popup
  grab) rather than QML-layer click tracking. Confirms PR #93's pivot
  and forbids reverting to a shield/PanelWindow approach without a
  documented compositor-spec change.
- **FR-002** — Menu walk results must be eagerly cached on every
  `niri WindowsChanged` event for new PIDs, in a background task that
  does NOT block the focus-rendering path. Cache TTL extended to the
  PID's lifetime (evicted on `WindowClosed`).
- **FR-003** — Bridge subscribes to `org.a11y.atspi` accessible-tree
  change signals (`ChildrenChanged`, `StateChanged`) per cached app,
  and invalidates / re-walks the affected subtree without a focus
  event. Defeats the Firefox-lazy-realisation gap.
- **FR-004** — Visual treatment matches the noctalia-shell card
  vocabulary (`Color.mSurface` base, `Style.radiusL` corners, 1 px
  `Color.mOutline` border, hover row with `Color.mPrimary` 2 px accent
  stripe). Style decisions are bound to the shell's `Style` + `Color`
  singletons — no raw hex / fixed sizes in the plugin.

### 3.2 Governance — CLAUDE.md update

- **FR-005** — Add a "Drift detection" section to the project CLAUDE.md
  with explicit triggers: two consecutive failed iterations on the
  same bug, ≥ 5 hotfix releases in a 24 h window, any "fix" comment
  citing the previous fix's failure mode by version number.
- **FR-006** — Add a "Decision tree" section listing the standard
  escalation paths when a drift trigger fires: codex review, parallel
  research swarm (3–5 specialised subagents), spec-driven redesign,
  upstream-source code reading. Each path has a concrete entry-point
  command.
- **FR-007** — Cite the agent-drift / alignment material from
  `~/Documents/Notes/AGENTS.md` and adjacent files; surface the
  relevant principles inline so no future Claude session needs to
  re-discover them.
- **FR-008** — Add a "When to redesign vs. keep iterating" checklist
  driven by the v1.0.5..v1.0.12 case study (seven plugin releases for
  one dismiss bug). The checklist must be referenceable from the
  decision tree.

### 3.3 Research swarm — execution

- **FR-009** — Spawn ≥ 4 specialised subagents in parallel, each on a
  distinct domain:
  - Wayland popup dismiss SOTA on niri / wlroots
  - AT-SPI eager-walk / cache-invalidation strategies in upstream
    GNOME / KDE menu mirrors
  - Quickshell + noctalia-shell visual idiom (Style/Color/Component
    inventory)
  - Agent-drift + alignment principles from the vault notes
- **FR-010** — Each subagent returns a synthesis (< 400 words) with
  concrete pattern recommendations + at least 2 citations (upstream
  source / blog / paper / vault note).
- **FR-011** — A synthesis pass collates the four reports into a
  single implementation plan covering FR-001..FR-008.

## 4. Success criteria

- **SC-001** — Pedro can dismiss the popup with one click outside it,
  every time, on niri-on-AMD, with no log entries reporting a missed
  click (verified via the v1.0.10 `[appmenu] shield press` debug log
  pattern — promoted to a `[appmenu] popup_done` log in v1.0.13).
- **SC-002** — First-focus latency from `niri WindowFocusChanged` to
  popup-visible is ≤ 16 ms on Firefox after the app has been open for
  ≥ 5 s (preload window has elapsed).
- **SC-003** — Firefox `Bookmarks / Profiles / Tools / Help` all show
  non-zero children counts in `~/.cache/noctalia-appmenu/active.json`
  for any window that has been open ≥ 5 s.
- **SC-004** — A blind side-by-side screenshot of the appmenu popup
  and the noctalia Calendar popup is indistinguishable in
  radius/border/spacing/typography (Pedro-judged).
- **SC-005** — The CLAUDE.md decision tree is referenced (and
  followed) on the next two non-trivial bugs in this repo. Measured by
  presence of "drift trigger fired" markers in the agent transcripts
  / commit messages.
- **SC-006** — The next 90 days of plugin releases average ≤ 2
  iterations per shipped bugfix (vs. the v1.0.5..v1.0.12 average of
  seven).

## 5. Out of scope

- Cross-compositor support beyond niri (KWin, Hyprland, Sway). Stays
  niri-first until the niri experience meets SC-001..SC-004.
- Replacing the AT-SPI substrate with `com.canonical.dbusmenu` (per
  ADR-0024).
- Visual themes other than Catppuccin Mocha (the user-CLAUDE.md
  default).

## 6. Assumptions

- noctalia-shell continues to expose its `Style` + `Color` singletons
  to plugins (`qs.Commons`, `qs.Services.UI`). Confirmed against the
  current `2026-04-16` shell snapshot.
- Quickshell `PopupWindow` remains the canonical xdg_popup wrapper.
- `niri-ipc` continues to publish `WindowsChanged` / `WindowClosed` /
  `WindowFocusChanged` events on the public event-stream. Verified
  against `niri-ipc 26.4`.
- The vault path `~/Documents/Notes/AGENTS.md` (and any sibling
  drift / alignment notes) is present on Pedro's host. Verified
  `ls ~/Documents/Notes` returns `AGENTS.md` at the root.

## 7. Open questions

- [NEEDS CLARIFICATION: are AT-SPI `ChildrenChanged` signals reliable
  enough on Firefox to drop the per-PID TTL entirely, or do we keep
  a long TTL as belt-and-braces? Research swarm Q2 will answer.]
- [NEEDS CLARIFICATION: should the visual polish be a separate v1.1.0
  release (theme breaking change?) or rolled into v1.0.13?]
- [NEEDS CLARIFICATION: governance scope — apply the new CLAUDE.md
  drift rules to *this repo only*, or also enforce them globally via
  `~/.claude/CLAUDE.md`? Probably this-repo-only first; extract to
  global on second proof point.]

## 8. References

- `~/Documents/Notes/AGENTS.md` (vault, drift/alignment material)
- `noctalia-shell/Modules/MainScreen/PopupMenuWindow.qml`
  (noctalia-shell upstream dismiss pattern)
- `noctalia-shell/Widgets/NPopupContextMenu.qml` (xdg_popup wrapper)
- `noctalia-shell/Modules/Cards/CalendarHeaderCard.qml` (visual
  vocabulary)
- ADR-0007..ADR-0027 (this repo)
- handoff.md (this repo) — v1.0.5..v1.0.6 drift case study
- PR history: #82 #83 #84 #85 #86 #87 #88 #89 #90 #91 #92 #93
