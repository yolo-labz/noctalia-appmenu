# ADR-0017 — plugin manifest schema (noctalia-shell v1)

- **Status:** Accepted (2026-05-05)
- **PR:** #24
- **Released in:** v0.1.5
- **Supersedes:** the speculative manifest in v0.1.0..v0.1.4

## Context

The plugin manifest at `plugin/manifest.json` was authored against an
imagined `https://docs.noctalia.dev/schemas/plugin-manifest-v1.json`
schema before noctalia-shell's actual loader was inspected. v0.1.0..v0.1.4
shipped:

```json
"entrypoint": "BarWidget.qml",
"widgetIds": ["AppMenu"]
```

Live noctalia-shell (commit `a7c7241`, the version Pedro runs) loads
plugins via `Services/Noctalia/PluginRegistry.qml::validateManifest`
followed by `Services/Noctalia/PluginService.qml`. The actual
schema is:

- **Required (hard-fail in `validateManifest`):** `id`, `name`,
  `version` (must match `/^\d+\.\d+\.\d+$/`), `author`, `description`,
  `entryPoints` (must exist as a key, must be an object).
- **Recognised entry-point keys** (any subset; `PluginService.qml`):
  - `entryPoints.main` — `Main.qml` instantiated in `pluginContainer`
  - `entryPoints.barWidget` — registered via
    `BarWidgetRegistry.registerPluginWidget`
  - `entryPoints.desktopWidget` —
    `DesktopWidgetRegistry.registerPluginWidget`
  - `entryPoints.controlCenterWidget` —
    `ControlCenterWidgetRegistry.registerPluginWidget`
  - `entryPoints.launcherProvider` —
    `LauncherProviderRegistry.registerPluginProvider`
- **Optional `metadata`** is passed verbatim to every registrar's third
  argument. The only key actively consumed by the bar registry is
  `cpuIntensive` (boolean) — `label` / `icon` / `settingsPath` are
  conventions, not validated.
- **Bar widget id is derived:** `BarWidgetRegistry.qml:455` produces
  `"plugin:" + pluginId`. For us: `plugin:noctalia-appmenu`. Users
  reference this id in their noctalia-shell `bar.widgets.<section>`
  list. Sibling keys on the entry become per-instance settings.
- **Silently-ignored fields:** `compositors`, `quickshellMin`,
  `noctaliaMin`, `requires`, `widgetIds` — none are read by either
  the registry or the service. We keep them as informational metadata
  but they have no runtime effect.

The fields we shipped (`entrypoint` singular, `widgetIds`) are NOT in
the loader's vocabulary. Net effect: every host that integrated
v0.1.0..v0.1.4 saw `validateManifest` either reject the manifest (no
`entryPoints` field) OR silently no-op (no recognised entry point) —
the plugin loaded into `installedPlugins` but never registered a bar
widget. Combined with ADR-0016 (niri schema), the v0.1 widget was
non-functional on every host.

## Decision

v0.1.5 ships a manifest that matches the live loader exactly:

```json
{
  "id": "noctalia-appmenu",
  "name": "AppMenu",
  "version": "0.1.0",
  "author": "Pedro H S Balbino <phsb5321@gmail.com>",
  "description": "macOS-style global menu …",
  "entryPoints": {
    "barWidget": "BarWidget.qml"
  },
  "metadata": {
    "label": "AppMenu",
    "icon": "menu",
    "cpuIntensive": false
  }
}
```

CI gains a `manifest schema check` step in `Plugin — qmllint`
(`.github/workflows/ci.yml`) that mirrors `validateManifest` plus
asserts:

- `entryPoints.barWidget` points at a file that exists.
- The legacy `entrypoint` (singular) field is absent.
- The legacy `widgetIds` field is absent.

Both negative checks defend against accidentally re-introducing the
v0.1.0..v0.1.4 schema drift.

## Consequences

- Bar widget id is **`plugin:noctalia-appmenu`**, not `AppMenu`.
  Downstream consumers reference it accordingly:
  ```nix
  programs.noctalia-shell.settings.bar.widgets.left = [
    { id = "plugin:noctalia-appmenu"; }
  ];
  ```
- The HM module's `widgetPlacement` option is now **declared but
  unconsumed in v0.1**. Wiring it to noctalia-shell's settings module
  would require cross-module coordination; deferred to v0.2 alongside
  the DBusMenu mirror so the same module owns plugin install +
  bar-widget placement together. Documented in `nix/module.nix`.
- The schema CI step is fast (jq + grep, ~2s) and runs in the
  existing `Plugin — qmllint` job — no new workflow, no new
  required-status-check.
- Silently-ignored fields (`compositors`, `quickshellMin`, etc.)
  remain in the manifest as informational metadata. If noctalia-shell
  starts validating them in a future release, the manifest is already
  truthful.

## Alternatives considered

- **JSON Schema with `ajv`:** more general but adds a Node toolchain
  dependency. The `validateManifest` logic is small enough to mirror
  in jq + bash for zero extra deps.
- **Auto-add the widget to `bar.widgets`:** would require this HM
  module to read/write noctalia-shell's settings — tight coupling
  across two flake inputs. Deferred to v0.2 when the DBusMenu mirror
  also lands and one place owns the full integration.
- **Submit a JSON Schema upstream to noctalia-shell:** worth doing,
  but blocks v0.1.5. File as v0.2 follow-up.

## References

- `noctalia-shell` commit `a7c7241`, files
  `Services/Noctalia/PluginRegistry.qml` (validate) and
  `Services/Noctalia/PluginService.qml` (load).
- `Services/UI/BarWidgetRegistry.qml:455` for the
  `plugin:<id>` derivation.
- v0.1.5 release notes.
