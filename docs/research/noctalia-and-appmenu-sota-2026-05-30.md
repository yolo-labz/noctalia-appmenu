# Research — Noctalia internals + Wayland app-menu / shell SOTA (2026-05-30)

Synthesis of a 4-agent research sweep (2 reading local Nix-store source, 2 web).
Grounds the appmenu plugin's architecture + the bar-integration work, and
benchmarks the project's substrate decisions against the external state of the
art.

> **Web-research path:** the self-hosted SearXNG (`server.tailf59220.ts.net:8443`)
> was unreachable / all-engines-timed-out this session, so web findings came
> from canonical sources (project GitHubs, KDE/GNOME/freedesktop docs,
> wayland.app, Mozilla/Chromium trackers). Flagged per the web-research mandate.

---

## TL;DR (the five things that matter)

1. **The project's substrate decisions are externally confirmed correct — and
   ahead of the field for wlroots/Smithay Wayland.** No other global-menu
   project uses AT-SPI as the menu *source*; every comparable one
   (vala-panel-appmenu, Fildem, helloSystem, KDE's own applet) is
   `dbusmenu`+Registrar and X11/KWin-bound. Even KDE Plasma's global menu is
   flaky-to-broken on Wayland. The **AT-SPI-first → `.desktop`-fallback** ladder
   (ADR-0024 + ADR-0031) is the only design that yields real menus on niri.
2. **⚠️ Noctalia v5 is a ground-up C++ rewrite that DROPS Qt/QML** (announced
   2026-04-24). v4 is frozen (maintenance only); **all QML plugins must be
   ported** when v5's new plugin API lands. Our **Rust bridge is insulated**
   (host/compositor-agnostic); only the thin QML `BarWidget.qml` is v4-bound.
3. **Quickshell is the 2026 center of gravity for high-end shells** (end-4
   14.7k★, caelestia 9.7k★, noctalia 7.1k★ — all QML/Quickshell). The modern
   look = floating bars + **capsule/pill widgets** + Material-You wallpaper
   theming + token-only styling. This *validates the current bar-integration
   direction* (capsule widgets matched to `Color.m*`/`Style.*` tokens).
4. **Strategic upside of betting on AT-SPI:** when libcosmic/Iced and GTK ship
   their AccessKit→AT-SPI registration (in progress, 2025–26), those apps gain
   real menus **through the existing walker, zero code change** — the ADR-0031
   "passive AccessKit compatibility" follow-up.
5. **One ADR-0032 amendment owed:** the "no Wayland property" phrasing is
   technically imprecise (`gtk_shell1.set_dbus_properties` *does* carry
   `menubar_path`) but the conclusion holds — the field is compositor-private
   (client→compositor, no read-back event) and niri ships no consumer
   (0 source hits). A one-line note pre-empts a future agent reopening the
   ladder item.

---

## 1. How Noctalia works (Quickshell shell, v4.7.x)

**What it is.** A Quickshell-based Wayland desktop *shell* (bar + dock +
notifications + OSD + lock screen + launcher), QML/Qt, MIT, ~7.1k★, "quiet by
design." Ships its own Quickshell fork `noctalia-qs`. Multi-compositor by design
(niri, Hyprland, Sway, Scroll, Labwc, MangoWC via a `CompositorService`
abstraction) — unusually broad vs Hyprland-locked peers.

**Top-level + rendering model.** `shell.qml` boots services in a strict DAG
(core → settings → visual → system → UI → plugins-last). The **entire bar +
its popups render as ONE `PanelWindow` per screen** — a deliberate
single-surface design so Wayland surface-damage can be batched (avoids the
full-output redraw flicker on AMD this project hit in PR #47–#52).

**The Bar.** Three sections (`left`/`center`/`right`, or top/bottom on vertical
bars), each a `Repeater` over a `ListModel`; delegates are preserved when only
settings change, rebuilt on structural change. Density presets
(`mini`/`compact`/`default`/`comfortable`/`spacious`) drive `barHeight` (21–47px)
+ `capsuleHeight` + `barFontSize`.

**Design system — two singletons:**
- `Commons/Color.qml` — Material-3 role tokens, `m*`-prefixed to avoid QML
  signal collisions: `mPrimary/mOnPrimary`, `mSecondary`, `mTertiary`,
  `mSurface/mOnSurface`, `mSurfaceVariant/mOnSurfaceVariant`, `mError`,
  `mOutline`, `mShadow`, **`mHover/mOnHover`**. Wallpaper-derived dynamic color
  (Material-You / matugen-class) + preset schemes (Catppuccin, Gruvbox, Nord,
  Tokyo-Night, Rosé Pine, Dracula, Ayu, Kanagawa, Noctalia-default). Per-token
  `ColorAnimation` (≈750ms) on scheme change. `colors.json` is file-watched.
- `Commons/Style.qml` — font sizes (`fontSizeXXS..XXXL`), **weights**
  (`fontWeightRegular 400 / Medium 500 / SemiBold 600 / Bold 700`), radii
  (`radiusXXXS..L`; **`radiusM` = 16px** is the capsule radius), margins
  (`marginXXXS..XL`; `marginS` = 6px is the standard spacing/padding),
  `barHeight`, **`capsuleHeight`** (≈0.65–0.9×barHeight), animation durations
  (`animationFast` 150ms / `Normal` 300ms / `Slowest` 750ms), and the
  **capsule system**: `capsuleColor` (= `mSurfaceVariant` + user opacity),
  `capsuleBorderColor` (`mPrimary` if outline on, else transparent),
  `capsuleBorderWidth` (1px). Everything scales by `uiScaleRatio` +
  `Settings.data.bar.fontScale`.

**Reusable widget components** a plugin should mirror: `NText` (default Medium
weight, `mOnSurface`, elide), `NIcon` (Tabler font), **`BarPill`/
`BarPillHorizontal`** (the canonical capsule container — `radiusM`,
`capsuleColor` bg, `mHover` on hover, `animationFast` transition),
`NScrollText` (hover-scroll long text).

**Plugin system.** Manifest-based (`manifest.json`: id/name/version/author/
description + `entryPoints.barWidget`). `PluginRegistry` scans
`~/.config/noctalia/plugins/*/manifest.json`; `PluginService` loads enabled
ones (can git-sparse-checkout to install). Bar-widget contract — the shell
injects into the widget root: `screen` (ShellScreen), `widgetId`, `section`,
`sectionWidgetIndex`, `sectionWidgetsCount`, `pluginApi`. Settings via
`Settings.getBarWidgetsForScreen(screen.name)[section][index]`. ~100 plugins
exist (a real ecosystem). **This is exactly the contract our `BarWidget.qml`
implements.**

**"Native-feel" rules (what makes a widget integrate vs look bolted-on):**
token-only theming (`Color.m*` + `Style.*`, never raw hex); honor
density + `fontScale`; claim layout space per the section contract; reuse the
capsule vocabulary (`capsuleColor`/`radiusM`/`capsuleHeight`) + `mHover`
interaction; resolve icons via the host (`Quickshell.iconPath`). **Our recent
v1.0.26/27 widget work already follows this; the per-item-capsule pivot in
flight aligns it further with the BarPill convention.**

---

## 2. Quickshell (the substrate, v0.3.0)

QML/Qt runtime for Wayland shells; `qs -c <config>` loads `shell.qml`;
hot-reload via a `Reloadable` model. Maintained by **outfoxxed**, ~quarterly
releases, still 0.x (API churn).

Types we rely on / must know:
- **`PanelWindow`** — layer-shell window (anchors, `exclusiveZone`,
  `WlrLayershell` layers Background/Bottom/Top/Overlay, `keyboardFocus`).
- **`PopupWindow`** (`grabFocus`) — but **popup-grab is compositor-dependent**:
  Hyprland has `HyprlandFocusGrab`; **niri (Smithay, not even wlroots) has no
  equivalent** → the outside-click-dismiss saga (v1.0.5–v1.0.12) is a direct
  symptom; the project moved dropdowns to a full-screen `PanelWindow` shield.
- **`IpcHandler`** + `qs ipc call <target> <fn> <args>` — how the bridge pushes
  `active.json` to the widget. (CLI: `qs -c noctalia ipc call appmenu update <json>`;
  `ipc show`/`call`/`wait`/`listen`/`prop`; instance via `-i`/`--pid`. Note:
  **`qs … ipc reload` is NOT a valid subcommand** — reload = config-watch or
  service restart.)
- **`Quickshell.Io.FileView`** — reactive file read; `text()` is a CALL
  (ADR-0021); in-place truncating writes keep the inode stable for inotify.
- **`ToplevelManager`/`Toplevel`** — focused-window tracking, BUT `Toplevel`
  exposes **no `pid`** (ADR-0002) → we resolve focus via niri IPC instead.
- **`Quickshell.DBusMenu.DBusMenuHandle` is `QML_UNCREATABLE`** — QML cannot
  bind to an arbitrary `(busName, objectPath)`. **This single constraint is why
  the project ships a Rust sidecar** (ADR-0003/0007) instead of a pure-QML
  menu.
- **qmlcache mtime trap on Nix** — store mtimes are the 1969 epoch, so a
  wall-clock-dated `.qmlc` from the prior release wins the freshness check and
  silently loads stale QML. The release skill nukes
  `~/.cache/noctalia-qs/qmlcache/` every deploy.

---

## 3. Global / app-menu SOTA — and where we sit

**Protocols, ranked for niri (wlroots/Smithay, no appmenu protocol):**

| Rank | Approach | On niri | Why |
|---|---|---|---|
| **1** | **AT-SPI tree-walk** (ADR-0024) | **Works — only real-menu source** | Cross-toolkit a11y bus; Qt5/6 + GTK3 (a11y on) expose `MENU_BAR`; needs nothing from the compositor. |
| **2** | **`.desktop`-action fallback** (ADR-0031) | **Works — the honest floor** | Real `[Desktop Action]`s + niri window controls for the no-AT-SPI-menubar majority. |
| 3 | `org.gtk.Menus` / GMenuModel | **Dead end** (ADR-0032) | GTK4 exports no menubar model (0/5 measured); discovery (`gtk_shell1.set_dbus_properties`) is compositor-private, no read-back on niri. |
| 4 | `dbusmenu` + `org_kde_kwin_appmenu` Registrar | **Dead end** | The appmenu Wayland protocol is **KWin-only**; niri/Sway/Hyprland/COSMIC don't implement it (the v0.2 failure that forced the ADR-0024 pivot). |

**Per-desktop reality (2026):** KDE Plasma ships a global-menu widget that works
on X11 but is flaky/broken on Wayland (KDE bug 424485 still REOPENED, GTK+KWin
"haven't agreed on how an app says 'this is my D-Bus name'" after 6 years).
GNOME removed its app menu in 3.32 (2019) — in-window hamburger by design.
Third-party menus (vala-panel-appmenu, Fildem-v2) are all `dbusmenu`+module+
Registrar, X11-bound, and hit the *same coverage wall* ("works for Chrome/GIMP/
LibreOffice; not most GNOME apps or Firefox") — which is exactly why a fallback
tier matters. macOS gets 100% coverage only because one vendor owns toolkit+WM+
OS (the NSMenu responder chain); Linux structurally can't.

**Electron/Chromium/Firefox:** menus need the a11y flag
(`--force-renderer-accessibility`) or `accessibility.force_disabled=0`;
otherwise our fallback applies. (New 2025: Chromium can export a Wayland
dbusmenu — but only to `org_kde_kwin_appmenu` = KWin; a no-op on niri.)

**Verdict:** the AT-SPI-first + `.desktop`-fallback design is **sound and
genuinely uncommon** for this compositor class, and it's *forward-compatible* —
AccessKit is the basis of the next-gen Linux a11y architecture (GTK 4.18 merged
an AccessKit backend in 2025; a push-based Wayland a11y protocol is prototyped/
STF-funded). As libcosmic/GTK light up AT-SPI, our walker inherits them free.

---

## 4. Wayland shell/bar landscape (2026)

| Toolkit | Runtime | Model | 2026 status |
|---|---|---|---|
| **Quickshell** | C++/QML | declarative QML scene | **momentum toolkit**; powers end-4/caelestia/noctalia; v0.3.0; 0.x churn |
| waybar | C++/GTK3 | config (JSON+CSS) | most-used; conservative default |
| eww | Rust/GTK3 | declarative (`yuck`+SCSS) | mature, portable |
| AGS→Astal | Vala+JS/Py/Lua | imperative+reactive (JSX) | the GTK/JS standard post-rewrite |
| ironbar | Rust/GTK4 | config+Lua | quietly very active |
| fabric | Python/GTK3 | imperative | Python alternative to Astal |
| HyprPanel | TS/Astal | config+TS | **ARCHIVED 2026** |
| gBar | C++/GTK3 | compiled config | **stale (2024)** |

**Design conventions that read as "modern/native" (2026):** floating detached
bars (inset, rounded); **capsule/pill widgets** grouped L/C/R; Material-You
wallpaper-derived color (role tokens, not hex); per-widget hover-expand;
restrained native easing (QML `Behavior`/`NumberAnimation` — a reason Quickshell
shells look smoother than GTK-CSS bars); icon fonts (Tabler/Material Symbols).
**niri** (Rust/Smithay, 24.7k★, scrollable-tiling): ships no bar; stable JSON
IPC + event-stream explicitly built for bars (listen, don't poll — how our
bridge tracks focus + the only reliable way to get the focused PID on Wayland).

---

## 5. Implications + recommended follow-ups for THIS project

- **Bar-integration (current task):** the research validates capsule/token-based
  widgets as the native look. Continue matching `BarPill` (`capsuleColor`,
  `radiusM`, `capsuleHeight`, `mHover`, `fontWeightMedium`, `animationFast`).
- **ADR-0032 amendment (recommended):** note that `gtk_shell1.set_dbus_properties`
  carries `menubar_path` but is compositor-private (no read-back; niri ships no
  consumer) so the dead-end conclusion is unchanged.
- **Plan for Noctalia v5 (C++, no QML):** the bridge is durable; the QML widget
  is v4-bound and will need a port to v5's (TBD) plugin API. Don't over-invest
  in deep QML cleverness that v5 will obsolete; keep the widget thin. Track v5's
  plugin-API announcement.
- **Free wins on the horizon:** libcosmic/GTK AccessKit→AT-SPI registration will
  auto-upgrade those apps from fallback to real menus via the existing walker
  (ADR-0031 follow-up #2 / issue #157).
- **niri integration is already idiomatic** (event-stream listen, `niri msg
  action` for window controls). No change needed.

## Sources
Local Nix-store source: `noctalia-shell-2026-05-15` + `noctalia-qs`/`quickshell`
type catalogs. Web (canonical, SearXNG down): KDE bug 424485, wayland.app
(kde-appmenu / gtk-shell), GTK/GNOME/freedesktop a11y docs, AccessKit, Chromium
commit 9c30fb3, Mozilla bug 1419151, Apple Cocoa menu docs, quickshell.org,
docs.noctalia.dev, "Announcing Noctalia v5" (2026-04-24), niri IPC docs, GitHub
repos (waybar/eww/astal/ironbar/fabric/caelestia/end-4). Full agent reports +
citations in the session transcript.
