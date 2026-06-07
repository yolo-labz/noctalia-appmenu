# ADR-0035 — Lazy/reveal-locked menubars serve the desktop fallback (Firefox on niri)

- **Status:** accepted
- **Date:** 2026-06-06
- **Deciders:** Pedro H S Balbino
- **Supersedes:** [ADR-0034](ADR-0034-lazy-menu-expand-on-demand.md) (expand-on-demand) for the Firefox/all-childless case
- **Related:** ADR-0024 (AT-SPI substrate), ADR-0031 (desktop fallback), ADR-0032 (org.gtk.Menus)

## Context

ADR-0034 added expand-on-demand: when a Firefox top-level menu had zero
AT-SPI children, the bridge fired `Action.DoAction(0)` ("click") to realise
them. v1.0.33→1.0.35 iterated on that path (flash dedup, skip fast-recover).
Pedro reported escalating breakage — culminating in a **permanent duplicate
menubar**: clicking a menu in our bar reveals Firefox's own menubar, which
then **stays on screen**.

## Measurement (live, 2026-06-06, Firefox 153 on niri)

The `DoAction(0)` that realises the children also **visually opens Firefox's
menubar**, and it cannot be undone:

- `tool bar "Menu Bar"` `SHOWING` state was already `true` and **stayed
  `true`** after a second `DoAction(0)` (the "collapse"): the dropdown
  closes, the menubar does not re-hide.
- `wtype -k Escape` → menubar still `SHOWING=true`.
- `wtype -k Alt_L` (the standard Firefox menubar toggle) → still `true`.
- Focusing **away** from Firefox (to ghostty) → still `true`.
- `xulstore.json` has `toolbar-menubar autohide:"true"` — it *should* hide,
  but a `DoAction`-reveal pins it.

So on niri there is **no client-side way to read Firefox's lazy menu items
without leaving its menubar visibly pinned.** AT-SPI's only action is
`"click"` (`NActions = 1`); there is no realise-without-open.

## Decision

**A walked menubar whose top-levels are ALL childless is treated as "no
readable menu" and serves the [desktop fallback](ADR-0031-desktop-fallback.md)**
— same honest fallback as libcosmic (#157). `proxy::is_lazy_unreadable`
(`!children.is_empty() && children.iter().all(|c| c.children.is_empty())`)
gates the `source = "atspi"` decision; such a menubar falls through to
`desktop-fallback`.

Eagerly-populated menubars (Qt6 / KDE — Anki, Okular, Kate; their top-levels
carry items at walk time) are unaffected and keep `source = "atspi"`. A
*partially* lazy menubar (some top-levels populated) is still served — only
all-childless triggers the fallback.

This **reverses ADR-0034's expand-on-demand as the default** for the
all-childless case: reading the menu is not worth pinning a duplicate
menubar. (The drift doctrine requires an ADR for an architectural backtrack
— this is it. ADR-0034's expand subcommand remains in the binary for the
partial-lazy / non-reveal-locked case and as a building block, but the
bridge no longer auto-invokes it for all-childless Firefox.)

## Consequences

- **Positive:** the bar is clean and honest for Firefox — no reveal, no
  duplicate menubar, no per-click chaos. Firefox shows its `.desktop`
  fallback (name + New Window / New Private Window + niri window controls).
- **Negative / cost:** Firefox loses File/Edit/View *in the bar*. Pedro's
  original ask ("where's View/Edit") is **not** satisfiable cleanly via
  AT-SPI on niri — see below. Documented as a niri-limitation in the README.

## The real fix (not reachable from the bridge)

A genuinely clean Firefox global menu on niri needs **niri to advertise the
`org_kde_kwin_appmenu_manager` Wayland global**, after which Firefox ≥138
exports its menu as `com.canonical.dbusmenu` **data** (no visual reveal),
which the bridge/shell renders. This is ~150 LoC in niri (Smithay bindings
exist) and **already implemented + working in a fork** (Naxdy/niri PR #46,
CertainLach) plus a maintainer-approved quickshell PR #484. ADR-0024's claim
that "niri's maintainer declined KWin protocols" is **unsubstantiated** (no
such statement exists; corrected in that ADR). So the upstream path is open,
not dead — but it is a niri compositor change + a strategic pivot (it would
make this AT-SPI bridge redundant for Qt/KDE/Firefox), so it is a separate,
Pedro-gated decision, not part of this fix.

## Verification

`proxy::is_lazy_unreadable` unit-tested (Firefox all-childless → fallback;
Qt eager → atspi; partial-lazy → atspi; empty → not flagged). `cargo test` +
`clippy -D warnings` green. End-to-end: Firefox now serves
`source = "desktop-fallback"`, no menubar reveal.
