# ADR-0031 — `.desktop` fallback menu for apps without an AT-SPI menubar

- **Status:** accepted
- **Date:** 2026-05-29
- **Deciders:** Pedro H S Balbino
- **Supersedes:** the v1.0.2 "honest-or-hidden" Empty posture (PR #47 /
  spec 011) for the no-AT-SPI-menubar case
- **Related:** ADR-0024 (AT-SPI substrate), ADR-0015 (v0.1 fallback-only),
  ADR-0029 (learned no-menubar skip), ADR-0030 (frame-scoped resolution)
- **Tracking:** spec 016, PR #160

## Context

ADR-0024 made AT-SPI the menu substrate. It works for apps that expose a
`MENU_BAR` accessible (Qt6 / GTK with the a11y bridge loaded — Anki,
Okular, Kate, Krita, LibreOffice). It cannot work for the large slice of
a daily-driver app set that exposes **nothing usable** on the a11y bus:

- libcosmic / Iced (`cosmic-files`, `cosmic-edit`) — no AT-SPI export
  upstream (issue #157);
- Electron without `--force-accessibility` (Obsidian, Feishin, VS Code,
  Slack);
- Chromium / Chrome, Firefox (flag-gated a11y);
- GTK4 `GtkPopoverMenuBar` apps (menu realised only when open in-window).

For all of these the bridge wrote `{ "menu": null, "source": "empty" }`
and the bar went blank — measured live across Pedro's running set
(ghostty, firefox-nightly, obsidian, feishin, chrome).

At v1.0.2 (PR #47, spec 011) we deliberately chose **honest-or-hidden**:
rather than show a synthesised menu, collapse the bar to zero width. The
reasoning was that the then-synthetic menu had included a `wtype`-driven
Edit submenu (Cut/Copy/Paste) that *lied* about keybindings — a real UX
trap (PR #44 removed it). "A blank-but-honest bar beats a lying one."

The conflation in that decision: the **lying** part was the faked
keystroke items, not the *idea* of a fallback. A fallback built only from
**real** actions, and **labelled** as a fallback, is honest. Meanwhile
the README already claimed (in three places) that a `.desktop`-derived
pseudo-menu existed — it did not. The bar was blank and the docs
overclaimed.

## Decision

Emit a labelled, identity-derived **desktop fallback** when AT-SPI
returns no menubar for a focused app. New `active.json`
`source = "desktop-fallback"`. The menu is built by
`bridge/src/desktop.rs::fallback_menu(app_id)`:

1. **AT-SPI first** (unchanged). A real menubar → `source = "atspi"`.
   The fallback is only reached on the `None` branch, so it never
   shadows a native menu.
2. **`.desktop`-enriched fallback** when `app_id` resolves to a
   freedesktop `.desktop` entry: an `<App>` button carrying the entry's
   `[Desktop Action]`s (or a synthesised *New Window* launch when it
   declares none) + *Quit*, and a `Window` button of niri controls.
3. **Minimal identity fallback** when `app_id` is known but no
   `.desktop` entry resolves: the app name + `Window` controls
   (`atspi::synthetic_menu`).
4. **Empty** only when no window is focused, `app_id` is empty, or the
   fallback is disabled.

### Why this is honest

- **Labelled.** `source = "desktop-fallback"` ≠ `"atspi"`. Consumers and
  users can tell it apart from a native menubar.
- **Real actions only.** `.desktop` actions launch the app's own `Exec`;
  window controls call `niri msg action`; *Quit* maps to niri
  *close-window* (never `SIGKILL`). No faked keystrokes — the v1 UX trap
  does not return.
- **Not a lie about coverage.** It is explicitly *not* the app's
  in-window menu, and the docs (README §App-menu fallback) say so.

### Opt-out

`desktop_fallback = false` in the bridge config restores the v1.0.2
honest-or-hidden Empty posture for users who prefer it.

## Security

The launcher never runs an `Exec` through a shell. `Exec` is tokenised
per the freedesktop spec, field codes (`%f %u %U …`) are stripped, and
`argv[0]` is spawned with the remaining args directly. The `active.json`
menu carries only opaque `<desktop-id>` / `<action-id>` tokens — never an
`Exec` string — and the click path re-resolves them against the trusted
XDG application dirs at click time. A tampered cache file can therefore
at worst launch a *different installed app*, never an arbitrary command.
This is the trust model every freedesktop launcher (rofi, wofi, fuzzel)
already operates under. The crate keeps `#![forbid(unsafe_code)]`.

## Consequences

### Positive

- The bar is useful for the modern-app majority instead of blank.
- Discovery honours `XDG_DATA_HOME` + `XDG_DATA_DIRS`, so NixOS profile
  dirs (`/run/current-system/sw/share`, `~/.nix-profile/share`,
  `/etc/profiles/per-user/$USER/share`) work with no hardcoded paths.
- Zero QML changes: the widget already renders `menu.children` and routes
  `service`/`path` clicks through `atspi-click`; the fallback reuses the
  existing `::synthetic` dispatch with two new `xdg` / `xdg-action`
  path prefixes.
- README is now accurate where it previously overclaimed.

### Negative

- A user could mistake a fallback for the real menu if they ignore the
  `source` field. Mitigated by docs and by the fallback's distinct,
  obviously-not-File/Edit/View shape.
- `.desktop` resolution can mis-map an unusual `app_id`. Mitigated by a
  priority ladder (direct id → StartupWMClass → Exec basename → fuzzy
  Name) and a 60 s memo; a wrong match degrades to a slightly-wrong
  launch label, never a wrong/destructive action.

## Verification

Built against the live host `.desktop` set: `google-chrome` (2 real
actions), `firefox-nightly` (3 actions incl. Profile Manager), `obsidian`
+ `feishin` (synthesised launch), `com.mitchellh.ghostty` (1 action). End
-to-end: running the bridge under a probe bus name with `firefox-nightly`
focused produced `active.json` `source = "desktop-fallback"`,
`menu = { children: [Firefox Nightly, Window] }` — the previously-blank
case fixed. Unit + integration tests cover parsing, field-code stripping,
resolution, priority, and menu shape.

## Follow-ups (not in this slice)

- `org.gtk.Menus` / `GMenuModel` D-Bus substrate (real menus for GTK apps
  that export them there rather than via AT-SPI).
- Passive COSMIC / AccessKit compatibility when libcosmic ships AT-SPI.
- niri window-action enrichment; locale-aware `.desktop` `Name[xx]`.

## References

- ADR-0024 — AT-SPI substrate
- spec 011 / PR #47 — honest-or-hidden (superseded here)
- PR #44 — removal of the `wtype` Edit submenu (the original UX trap)
- [freedesktop Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry-spec/latest/)
