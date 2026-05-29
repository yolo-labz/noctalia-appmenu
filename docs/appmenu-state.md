# appmenu — forward state

Living status doc for the universal app-menu effort. Updated by the
`/appmenu-forward` flow. Most recent entry on top.

---

## 2026-05-29 — desktop fallback wired (spec 016 / ADR-0031)

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

### Adversarial review

See PR #160 description. Codex `codex-rescue` adversarial pass requested
on the change set (AT-SPI regression risk, fake-fallback claims, unsafe
Exec handling, XDG/NixOS path assumptions, schema breakage). Any
BLOCKER/MAJOR resolved or documented before merge.

### Remaining risks

- `.desktop` resolution can mis-map an unusual `app_id` → wrong launch
  label (never a wrong/destructive action). Priority ladder + 60 s memo.
- A user ignoring the `source` field could mistake the fallback for a
  native menu. Mitigated by docs + the obviously-non-native shape.
