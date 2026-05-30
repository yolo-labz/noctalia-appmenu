# appmenu — forward state

Living status doc for the universal app-menu effort. Updated by the
`/appmenu-forward` flow. Most recent entry on top.

---

## 2026-05-29 — Window submenu enrichment (PR #164)

- **Branch:** `164-niri-window-actions` (PR #164). Ladder item 5.
- **Slice:** grow the fallback `Window` submenu from 5 flat leaves to a
  grouped set of niri-native column/monitor ops, so the fallback is a
  useful window controller, not just Close.
- **Changed:** `bridge/src/atspi.rs` — `synthetic_window_submenu` now:
  Close / Toggle Fullscreen / Toggle Floating · **Maximize Column** /
  **Center Column** / **Expand Column to Available Width** · Move to
  Prev/Next Workspace / **Move to Monitor Left/Right**, in 3
  separator-grouped sections. Added `synthetic_separator` helper +
  monitor-move icons (`go-previous`/`go-next`) to `niri_action_icon`.
  2 net new tests; layout test locks the exact rows + ids.
- **Source behaviour:** unchanged ladder. Window menu is part of both the
  enriched fallback and the minimal `synthetic_menu`.
- **Tests:** fmt + clippy + 95 lib tests green.
- **Smoke:** ✓ structure (probe shows the 3-group submenu) + ✓ all 5 new
  action names dispatch-valid (`niri msg action <name> --help`,
  non-mutating). ◐ live execution of the new actions NOT fired (would
  mutate the session layout); they ride the proven `dispatch_niri_action`
  path identical to the existing Close/Fullscreen actions.
- **QML:** separators render as dividers (`MenuRow` handles `type ==
  "separator"`; `SubmenuPopup` skips them) — confirmed, not assumed.
- **Review:** internal adversarial pass — action strings hardcoded (no
  injection), no shell, no schema/dep change. SHIP.
- **Follow-up:** next rung = `org.gtk.Menus` substrate (item 4, needs a
  spec first) or Noctalia provenance styling (item 6).

---

## 2026-05-29 — synthetic Window/Quit icons (PR #163)

- **Branch:** `163-synth-icons` (PR #163). Completes the #162 icon work.
- **Slice:** give the niri-synthesised leaves (Window submenu + Quit)
  standard freedesktop icon names so they theme alongside the
  `.desktop`-derived action leaves instead of rendering bare.
- **Changed:** `bridge/src/atspi.rs` — pure `niri_action_icon(action)`
  map (`close-window`→`window-close`, `fullscreen-window`→`view-fullscreen`,
  workspace moves→`go-down`/`go-up`; unmapped incl. floating → `""`),
  wired into `niri_leaf`. Applies to both the enriched fallback and the
  minimal `synthetic_menu`. 3 new tests.
- **Source behaviour:** unchanged. Icons additive only; unmapped actions
  stay iconless (never a misleading icon — `Quickshell.iconPath` also
  guards a theme miss).
- **Tests:** fmt + clippy + 94 lib tests (+3) green.
- **Smoke ✓:** probe `obsidian` — Window leaves carry
  `window-close`/`view-fullscreen`/`go-down`/`go-up` (Floating iconless),
  Quit carries `window-close`, New Window keeps the app icon.
- **Review:** internal adversarial pass (pure static-string mapping, no
  exec/IO/schema surface). SHIP.
- **Follow-up:** none for icons. Next ladder rung = `org.gtk.Menus`
  substrate (item 4) or Noctalia provenance styling (item 6).

---

## 2026-05-29 — desktop-fallback icons (PR #162)

- **Branch:** `162-fallback-icons` (PR #162). Ladder item 1 extension.
- **Slice:** populate `icon_name` in the desktop fallback from the
  `.desktop` `Icon=` keys. `MenuRow.qml` already renders `icon_name` via
  `Quickshell.iconPath` (FR-012) but the fallback always left it empty.
- **Changed:** `bridge/src/desktop.rs` — `icon` field on `DesktopEntry`
  + `DesktopAction`; parse `Icon=`; thread into `action_leaf`/`launch_leaf`
  + the root. Action icon falls back to the entry icon. 3 new tests.
- **Source behaviour:** unchanged ladder (atspi → desktop-fallback →
  empty). Icons are additive metadata only.
- **Tests:** fmt + clippy + 91 lib tests (+3) green.
- **Smoke ✓:** probe — `google-chrome`/`obsidian`/`com.mitchellh.ghostty`/
  `firefox-nightly` all carry `icon_name` on action/launch leaves +
  root, sourced from each app's own `.desktop Icon=`. Quit + separator
  correctly iconless.
- **Review:** internal adversarial pass (Codex not invoked —
  proportionate for an additive icon slice, no new exec/IO/security
  surface). SHIP.
- **Follow-up:** standard freedesktop icon names for the synthesised
  Window-control + Quit items (`window-close`, `view-fullscreen`,
  `application-exit`) — a separate polish slice.

---

## 2026-05-29 — desktop fallback wired (ADR-0031)

- **Branch:** `160-desktop-fallback` (PR #160)
- **Bridge version:** 1.0.25 (no bump this slice — feature only)
- **Slice:** wire a real `.desktop` fallback menu into the bridge so
  apps with no AT-SPI menubar stop producing a blank bar.

### Menu-source ladder (now)

| `source` | When | `menu` |
|---|---|---|
| `atspi` | App exposes a real `MENU_BAR` accessible (Qt6/GTK + a11y). | walked tree |
| `desktop-fallback` | Focused app, AT-SPI returned nothing, identity resolves. | non-null: app `.desktop` actions + niri window controls |
| `empty` | No focus / empty `app_id` / fallback disabled. | `null` |

AT-SPI is still tried first; the fallback only fires on its `None`
branch and never shadows a native menubar.

### Measured behaviour (verified 2026-05-29)

Probe + live bridge run (under a probe bus name, production bridge
untouched) on the real host app set:

- `firefox-nightly` focused → `active.json` `source=desktop-fallback`,
  `menu={children:[Firefox Nightly, Window]}`. Log:
  `walked atspi menubar … cached_negative=true` then
  `no AT-SPI menubar; emitting desktop-fallback menu top_level=2`.
- `google-chrome` → actions *New Window* + *New Incognito Window* + Quit.
- `firefox-nightly` → *New Private Window* + *New Window* +
  *Profile Manager* + Quit.
- `obsidian`, `feishin` (Electron, no actions) → synthesised *New Window*
  launch + Quit.
- `com.mitchellh.ghostty` → action *New Window* + Quit.

### Files changed

- `bridge/src/desktop.rs` (new) — `.desktop` parse, XDG discovery,
  `app_id` resolution, fallback-menu builder, safe argv launcher.
- `bridge/src/atspi.rs` — `pub(crate)` on 4 synthetic helpers;
  `dispatch_synthetic` gains `xdg` + `xdg-action` arms.
- `bridge/src/proxy.rs` — `MenuSource::DesktopFallback`; resolver calls
  `desktop::fallback_menu` on the AT-SPI-None branch.
- `bridge/src/config.rs` — `desktop_fallback: bool` (default true).
- `bridge/src/lib.rs` — `pub mod desktop;`.
- `bridge/examples/desktop_probe.rs` (new) — live probe.
- Docs: `README.md` (§App-menu fallback + table/caveats),
  `docs/adr/ADR-0031-desktop-fallback.md` (new),
  `docs/adr/README.md` index, `docs/reference/config.md`,
  `specs/004-project-completion/contracts/active-json-schema.md`,
  `plugin/BarWidget.qml` (comment only).

### Action execution (slice 1)

- Window controls + Quit → `niri msg action` (proven path).
- `.desktop` actions + default launch → safe argv spawn of the entry's
  own `Exec` (field codes stripped, **no shell**). Click path carries
  only opaque `<desktop-id>`/`<action-id>` tokens, re-resolved against
  trusted XDG dirs at click time.

### Unsupported app categories (honest)

- Apps the fallback cannot enrich beyond name + window controls: those
  with no `.desktop` entry (rare) → minimal identity fallback.
- The fallback is **not** the app's File/Edit/View menu. Native menus
  for Electron/Chromium/Firefox still need their per-app a11y flags.

### Follow-up tasks (not in this slice)

1. `org.gtk.Menus` / `GMenuModel` D-Bus substrate — real menus for GTK
   apps that export there rather than via AT-SPI.
2. Passive COSMIC / AccessKit compatibility when libcosmic ships AT-SPI
   (issue #157).
3. niri window-action enrichment (move-to-monitor, column ops).
4. Locale-aware `.desktop` `Name[xx]` selection.
5. Noctalia UI polish: distinct styling for `desktop-fallback` vs
   `atspi` (e.g. a subtle dot) so users see provenance at a glance.
6. Per-`.desktop`-action icon names (`icon_name` is currently empty).

### Adversarial review (Codex `codex-rescue`, 2026-05-29)

Verdict: FIX-FIRST. Findings + resolution:

- **BLOCKER — path traversal in click-time `xdg:`/`xdg-action:`
  re-resolution.** `dir.join("{id}.desktop")` accepted absolute / `..`
  ids, so a tampered `active.json` could spawn an arbitrary `.desktop`'s
  Exec. **Fixed:** `is_valid_desktop_id` (whitelist `[A-Za-z0-9._-]`,
  reject `..`/`/`/absolute/len>255) gates `resolve` + `resolve_in`. New
  tests `valid_desktop_id_*` + `resolve_in_rejects_path_traversal_*`.
- **MAJOR — eager publish transiently claimed `source="atspi"` with a
  null menu.** **Fixed:** eager source is now `empty` (truthful
  "loading") or an eagerly-built `desktop-fallback` for learned-skip
  apps; never optimistic `atspi`.
- **MAJOR — `XDG_DATA_DIRS` unset hardcodes `/usr/share`.** Code is the
  freedesktop-spec default and that env state never occurs in a niri
  session; **doc claim softened** (ADR-0031 + module doc).
- **MAJOR — learned-skip can shadow a newly-real menubar.** Pre-existing
  ADR-0029 behaviour, bounded by `RECHECK_TTL`; **documented** in
  ADR-0031 consequences. Strictly better than the prior blank bar.
- **Fixed (doc drift Codex caught):** schema contract now uses real wire
  keys (`focus_pid`, `focus_winid`, `type`, `menu_service/_path`,
  `menu: null`).

### Remaining follow-ups / known MINORs (not blocking)

- niri window-control dispatch uses `Command::new("niri")` (PATH), not
  `config.niri_binary` — pre-existing; the `atspi-click` subprocess does
  not load Config. Thread config into the click path in a later slice.
- No QML fixture exercises a `source="desktop-fallback"` / `xdg:` leaf
  payload yet (the widget provably ignores `source` and renders
  `menu.children`, which the fallback supplies identically). Add a
  qmltest fixture next slice.
- `exec_to_argv` drops unknown `%X` field codes (safe for a no-arg
  launch); revisit if a future slice passes file/URI args.

### Remaining risks

- `.desktop` resolution can misidentify an unusual `app_id` → wrong launch
  label (never a wrong/destructive action). Priority ladder + 60 s memo.
- A user ignoring the `source` field could mistake the fallback for a
  native menu. Mitigated by docs + the obviously-non-native shape.
