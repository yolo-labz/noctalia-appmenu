# ADR-0018 — bar-widget API contract

- **Status:** Accepted (2026-05-05)
- **PR:** #25
- **Released in:** v0.1.6

## Context

noctalia-shell's `BarSection.qml` instantiates registered plugin
widgets and injects per-instance state via property assignment:

```qml
property string widgetId: ""
property string section: ""           // "left" | "center" | "right"
property int sectionWidgetIndex: -1
property int sectionWidgetsCount: 0
property ShellScreen screen
property var pluginApi: null
```

If the widget root Item doesn't declare these properties, QML logs
`Cannot assign to non-existent property "widgetId"` (and the others)
on every instantiation. More importantly, the widget never lays out
correctly because its `widgetSettings`, `screenName`, `barPosition`,
and theme bindings all derive from these injected properties — the
upstream pattern is visible in `noctalia-shell/Modules/Bar/Widgets/
KeepAwake.qml` (and every other core widget).

v0.1.0..v0.1.5 of noctalia-appmenu's `BarWidget.qml` declared none
of them. With v0.1.5 (which fixed the manifest so the widget
actually registered), Pedro's journal showed:

```
WARN: Error: Cannot assign to non-existent property "widgetId"
WARN: Error: Cannot assign to non-existent property "sectionWidgetIndex"
WARN: Error: Cannot assign to non-existent property "sectionWidgetsCount"
WARN: Error: Cannot assign to non-existent property "pluginApi"
```

The widget was loaded into the registry but failed to compose into
the bar layout — the macOS-style menu strip stayed invisible.

## Decision

`BarWidget.qml` declares the full bar-widget contract verbatim from
the upstream `KeepAwake.qml` reference, plus the theme/typography
hooks core widgets use:

- **Required injected properties:** `screen`, `widgetId`, `section`,
  `sectionWidgetIndex`, `sectionWidgetsCount`, `pluginApi`.
- **Derived `widgetSettings`:** read via
  `Settings.getBarWidgetsForScreen(screenName)[section][index]` —
  same pattern as KeepAwake. Per-instance Nix-declared keys
  (`fallbackText`, `maxLabelWidth`, `showOnlyWhenFocused`) flow
  through this single source rather than landing on root as
  individually-mutable properties.
- **Theme integration:** `color: Color.mOnSurface` (was hardcoded
  `#cdd6f4`). Switching the predefined color scheme reflows the
  widget instantly — no rebuild required.
- **Typography:** `font.family: Settings.data.ui.fontDefault || "Inter"`
  + `font.pixelSize: Math.max(1, Style._barBaseFontSize *
  (Settings.data.bar.fontScale || 1.0))`. Density / fontScale changes
  in noctalia Settings are picked up live.
- **Sizing:** `implicitHeight: Style.barHeight` so the widget claims
  the full bar height (was `parent.height` — null on first paint
  before the bar attaches, caused intermittent layout misses).
- New imports: `qs.Commons` (for `Style`, `Color`, `Settings`) and
  `qs.Services.UI` (for the registry constants the contract relies
  on transitively).

## Consequences

- The widget now obeys the same theming and density signals as the
  core widgets — Pedro's `density = "comfortable"` + `fontScale = 1.2`
  setup applies to AppMenu without any per-plugin tuning.
- `widgetSettings.fallbackText` / `.maxLabelWidth` / `.showOnlyWhenFocused`
  must be set on the bar.widgets entry, NOT as root properties on
  the widget. Pedro's `shell.nix` already does this:
  ```nix
  { id = "plugin:noctalia-appmenu";
    fallbackText = "";
    maxLabelWidth = 200;
    showOnlyWhenFocused = true; }
  ```
- qmllint emits "Unqualified access" advisories for `Settings`,
  `Style`, `Color`, and `parent` (the QML singletons noctalia
  resolves at runtime from `qs.Commons`). qmllint can't see those
  modules without a full noctalia-shell project context — the
  warnings are advisory and do not block runtime.
- The `pluginApi` property is currently unused by AppMenu in v0.1.x.
  It's declared so noctalia can inject it without warnings, ready
  for v0.2's DBusMenu mirror to consume the plugin-IPC surface.

## Alternatives considered

- **Subclass a noctalia base widget** (e.g. `BarWidgetBase`): no such
  type is exported from `qs.Modules.Bar.Extras`. Each core widget
  duplicates the contract by hand — we follow suit.
- **Register a subclass via QML_ELEMENT and let noctalia find it:**
  noctalia's `BarWidgetRegistry` only consumes plain Components
  through `Qt.createComponent`. No type-system shortcut.
- **Use `Q_PROPERTY` introspection to dynamically bind injected
  properties:** complex, defeats QML's static-checking story, and
  doesn't avoid the `widgetSettings` derivation logic. The full
  declaration is ~10 lines — clarity wins.

## References

- noctalia-shell commit `9f8dd48`,
  `Modules/Bar/Widgets/KeepAwake.qml` (reference contract),
  `Services/Noctalia/PluginService.qml::createPluginAPI` (pluginApi),
  `Services/UI/BarWidgetRegistry.qml` (registration entry point).
- v0.1.6 release notes.
