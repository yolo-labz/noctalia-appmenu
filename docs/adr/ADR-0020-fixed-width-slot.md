# ADR-0020 — bar widget slot must be fixed-width

- **Status:** Accepted (2026-05-05)
- **PR:** #27
- **Released in:** v0.1.8

## Context

v0.1.7 (#26 / ADR-0019) made the BarWidget always render. Pedro
reported that the widget shows the placeholder `·` correctly when
no app is focused, but appears **invisible** as soon as `appId`
populates. The expected behavior was a stable text label tracking
focus changes; instead, focus events appeared to never reach the
widget visually.

Investigation traced the failure to noctalia-shell's
`Modules/Bar/Extras/BarWidgetLoader.qml:42`:

```qml
implicitWidth: isVerticalBar
  ? barHeight
  : getImplicitSize(loader.item, "implicitWidth")
```

QML's binding tracker registers the read of `loader.item` (the
property reference) when this expression evaluates, but does **not**
register reads of `loader.item.implicitWidth` performed inside the
function call. Function bodies opaque the dependency graph. Net
effect: `BarWidgetLoader.implicitWidth` re-evaluates only when
`loader.item` itself is re-assigned (component swap, plugin
hot-reload), never when its `implicitWidth` property merely changes.

Combined with v0.1.7's content-driven sizing
(`implicitWidth: label.implicitWidth + Style.marginM * 2`), the
sequence was:

1. First paint: `appId == ""`, `displayText = "·"`,
   `label.implicitWidth ≈ 3px`, `root.implicitWidth ≈ 21px`.
2. BarWidgetLoader caches its own `implicitWidth = 21px` and the
   bar's RowLayout reserves a 21px slot.
3. FileView's async `onLoaded` fires, `appId =
   "com.mitchellh.ghostty"`, `displayText` re-evaluates to the app id.
4. `label.implicitWidth` grows to ~160px; `root.implicitWidth` grows
   to ~178px.
5. **BarWidgetLoader's cached 21px does NOT update.** Bar slot stays
   at 21px.
6. Text overflows the slot. Sibling widgets (Clock) sit right where
   the overflow text would render. Net visual: text appears clipped,
   absent, or jumbled depending on positioning.

The placeholder visible in v0.1.7 was a happy accident — its width
was small enough that the cached slot could contain it. As soon as
real text arrived, the slot was undersized.

## Decision

The bar slot is reserved at `maxLabelWidth + marginM*2` regardless of
current content. The slot's first-paint width is therefore wide
enough to hold any displayText; the slot is never expanded by content
and never depends on noctalia's broken loader-cache reactivity.

```qml
implicitHeight: Style.barHeight
implicitWidth: maxLabelWidth + Style.marginM * 2

Text {
    id: label
    anchors.fill: parent
    anchors.leftMargin: Style.marginM
    anchors.rightMargin: Style.marginM
    verticalAlignment: Text.AlignVCenter
    horizontalAlignment: Text.AlignLeft
    text: root.displayText
    elide: Text.ElideRight
    …
}
```

The Text element switches from anchor-center to `anchors.fill: parent`
so its width matches the slot — `elide: Text.ElideRight` then actually
cuts overflowing app ids (longer than `maxLabelWidth - 2*marginM`)
instead of overflowing past the slot bounds.

## Consequences

- **Stable bar layout.** The slot width is constant; content updates
  only the rendered text, never the bar geometry.
- **Trade-off: empty placeholder shows in a wide slot.** When no app
  is focused, the `·` glyph sits left-aligned in a `maxLabelWidth`
  slot. Visually a wider gap than the slot's actual content. The
  alternative (content-driven width) re-introduces the loader-cache
  bug. Stable layout wins.
- **`maxLabelWidth` is now the authoritative slot width** — users
  who want a narrower AppMenu strip can shrink it via the bar widget
  entry's `maxLabelWidth` setting. Default 200 picked to match
  noctalia's `ActiveWindow` widget convention.
- **`Layout.maximumWidth` removed.** It was a no-op anyway (the root
  Item is not a Layout container), but its presence was misleading.

## Alternatives considered

- **Patch noctalia-shell to use a property binding instead of
  `getImplicitSize`:** correct fix but requires upstream PR + waiting
  for release. File as v0.2 follow-up.
- **Force a layout invalidation by toggling `loader.item.visible`
  from a Timer:** a hack that fights the bar host. Fragile across
  noctalia versions.
- **Use a Loader inside the widget that's `active: appId !== ""`:**
  same `loader.item` cache problem reappears one level down.
- **Compute width from `displayText` length × `font.pixelSize × 0.6`
  on first paint:** would let the slot grow with content but is
  font-metrics-fragile and doesn't help the post-load growth.

## References

- noctalia-shell commit 9f8dd48,
  `Modules/Bar/Extras/BarWidgetLoader.qml:42` (function-call binding).
- v0.1.7 release notes (always-visible widget that exposed the
  cache race once content sizing came online).
- Pedro's screenshots showing `·` placeholder visible but `appId`
  text invisible on 2026-05-05.
- v0.1.8 release notes.
