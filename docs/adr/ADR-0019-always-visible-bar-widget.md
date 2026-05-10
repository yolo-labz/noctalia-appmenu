# ADR-0019 — bar widget must always claim layout space

- **Status:** Accepted (2026-05-05)
- **PR:** #26
- **Released in:** v0.1.7

## Context

v0.1.6 shipped `BarWidget.qml` with this visibility gate:

```qml
visible: appId !== "" || fallbackText !== ""
```

The intent was reasonable: hide the widget when no application is
focused. The reality on Pedro's desktop was that the widget never
rendered, even after focus events flowed through `active.json`.

Inspection of `noctalia-shell/Modules/Bar/Extras/BarWidgetLoader.qml`
(commit 9f8dd48, line 45-49) revealed the gating mechanism:

```qml
visible: loader.item ? ((loader.item.opacity > 0.0) || …) : false
…
function getImplicitSize(item, prop) {
  return (item && item.visible) ? Math.round(item[prop]) : 0;
}
implicitWidth: isVerticalBar ? barHeight : getImplicitSize(loader.item, "implicitWidth")
```

`BarWidgetLoader` reads `item.visible` to decide its own
`implicitWidth`. When my widget was `visible: false` at first paint
(because `FileView { blockLoading: false }` had not yet read
`active.json`), the loader took 0 width. The bar's RowLayout snapped
its slot closed. Once `appId` became non-empty after FileView's async
`onLoaded`, the widget's `visible` flipped to true — but Qt's parent
RowLayout did not always reflow the cached slot width back, leaving
the widget invisible until the next layout invalidation
(workspace switch, monitor change, etc).

Net effect on Pedro's desktop: bar showed `Launcher → Clock` directly
with no AppMenu in between, even though `~/.cache/noctalia-appmenu/active.json`
was correctly populated and PluginService logged
`Plugin loaded: noctalia-appmenu` cleanly.

## Decision

The widget always renders. It always claims layout space.

- **Drop the `visible:` gate** entirely. Item.visible defaults to
  true, so `BarWidgetLoader.getImplicitSize` always returns the real
  computed width.
- **Always non-empty display string.** A new `displayText`
  property selects in priority: `appId` → `fallbackText` → `"·"`
  (CTP middle-dot placeholder). The widget always has a glyph to
  render, so `label.implicitWidth > 0` and the slot keeps a stable
  reserved-space.
- **Dim the placeholder.** `opacity: appId !== "" || fallbackText !==
  "" ? 1.0 : 0.45` makes the empty state read as "no focus" rather
  than as an app named `·`. Reads correctly once eyes adapt.

## Consequences

- **Stable bar layout.** The slot is always present. No reflow during
  focus changes; only the text content updates. This matches how
  noctalia's own `Clock` and `SystemMonitor` widgets behave.
- **Visual placeholder.** When no app is focused (e.g. session lock
  ends, or all windows closed), the bar shows `·` at half opacity
  rather than collapsing the slot.
- **The `showOnlyWhenFocused` widgetSetting is now functionally
  ignored** — kept on the entry shape for forward-compat in case v0.2
  re-introduces conditional hiding via a different mechanism (e.g.
  collapsing into an icon-only mode rather than `visible: false`).
- **Tradeoff considered:** auto-hiding the widget when no app is
  focused would be cleaner UX, but the BarWidgetLoader contract makes
  that race-prone. Stable-layout-with-placeholder is the
  least-surprising compromise that doesn't fight the bar host.

## Alternatives considered

- **Keep `visible: false` + force a layout invalidation on
  `appId` change:** would require touching the parent RowLayout from
  the widget, which violates the noctalia plugin contract (no
  reach-up into the bar host). Tried during diagnosis; fragile.
- **Set `implicitWidth: 1` when empty:** technically works but leaves
  a 1px gap with no visual indicator. Users would assume the widget
  is broken. The middle-dot placeholder is cheap signal.
- **Use `Loader { active: appId !== "" }`:** same race; `active`
  state still gates `implicitWidth` through `BarWidgetLoader`.

## References

- `noctalia-shell` commit 9f8dd48,
  `Modules/Bar/Extras/BarWidgetLoader.qml:45-49` (the gating logic).
- v0.1.6 shipped the broken visibility gate; this ADR retires it.
- Pedro's screenshots showing `Launcher → Clock` with no AppMenu slot
  on 2026-05-05.
- v0.1.7 release notes.
