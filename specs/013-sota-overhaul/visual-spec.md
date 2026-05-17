# noctalia-appmenu popup — visual style specification

Sources: `Commons/Style.qml`, `Commons/Color.qml`, `Widgets/NPopupContextMenu.qml`,
`Widgets/NDivider.qml`, `Widgets/NDropShadow.qml`, `Widgets/NText.qml`, `Widgets/NIcon.qml`.

---

## 1. Surface treatment

**Canonical noctalia pattern** (from `NPopupContextMenu.qml` lines 210-222):

```qml
// AppmenuPopupWindow.qml — menuBox Rectangle
color: Color.mSurface           // was: Color.mSurfaceVariant — too bright vs shell popups
border.color: Color.mOutline    // was: Color.mPrimary — primary accent border is non-canonical
border.width: Style.borderS     // 1 px at 1× scale; was hard-coded 2
radius: Style.radiusM           // 16 px at default radiusRatio; was Style.marginS (6 px)
```

Shell uses `Color.mSurface` (default `#070722`) for every popup background.
`Color.mSurfaceVariant` (`#11112d`) is reserved for *hovered rows*, not the container.
Border is `Color.mOutline` at `Style.borderS` — not the primary accent. The plugin currently
uses `Color.mPrimary` at `border.width: 2`; both are wrong.

No drop-shadow on the popup `Rectangle` itself. `NDropShadow.qml` is a `layer.effect`
applied to the *source item* via `Settings.data.general.enableShadows` — wire it only if you
wrap menuBox in an `NDropShadow` item identical to how SmartPanel does it (out of scope here).

Outer padding inside menuBox:

```qml
anchors.margins: Style.marginS   // 6 px — matches NPopupContextMenu flickable.margins
```

---

## 2. Row treatment

**Reference** (`NPopupContextMenu.qml` lines 248-270):

```qml
// MenuRow.qml — rowBg Rectangle
height: 28                             // fixed; Style.barHeight-Style.marginS is too variable
color: rowHover.containsMouse ? Color.mHover : "transparent"
radius: Style.iRadiusXS               // 8 px input radius; was row._xs (4 px — too small)

Behavior on color { ColorAnimation { duration: Style.animationFast } }
```

Row text/icon margins:

```qml
anchors.leftMargin: Style.marginM     // 9 px; matches NPopupContextMenu RowLayout
anchors.rightMargin: Style.marginM
spacing: Style.marginS                // 6 px between icon/toggle/label/chevron
```

Disabled rows: `opacity: 0.5` (NText sets `opacity: enabled ? 1.0 : 0.6`; the row wraps
so 0.5 from rowBg is the shell idiom for a disabled interactive surface).

**Separator** — use `NDivider` directly instead of a bare `Rectangle`:

```qml
// separator branch in MenuRow.qml
NDivider { anchors.leftMargin: Style.marginM; anchors.rightMargin: Style.marginM }
```

NDivider emits a gradient-faded `Color.mOutline` line at `Style.borderS` height, matching
every other separator in the shell. The plugin's current `opacity: 0.4` bare rectangle is
acceptable but inconsistent.

---

## 3. Typography

**Reference** (`NText.qml`, `NIcon.qml`, `NPopupContextMenu.qml`):

```qml
// MenuRow.qml — label — replace raw Text with NText
NText {
    text: (row.modelData?.label ?? "").replace(/_/g, "")
    pointSize: Style.fontSizeS        // 10 pt — same as NPopupContextMenu rows
    color: rowHover.containsMouse ? Color.mOnHover : Color.mOnSurface
    Layout.fillWidth: true
    Behavior on color { ColorAnimation { duration: Style.animationFast } }
}
```

**Icon**: replace `Image` with `NIcon` when the icon token is a named icon (from the shell
icon font). When the source is an XDG `icon_name` resolved via `Quickshell.iconPath`, keep
`Image` because `NIcon` only knows the noctalia icon font.

```qml
// icon_name slot — keep Image; size to match NIcon pointSize
Layout.preferredWidth: Style.fontSizeS * Style.uiScaleRatio
Layout.preferredHeight: Layout.preferredWidth
```

**Chevron** for submenu: `"›"` (U+203A) at `Style.fontSizeS`, `Color.mOnSurfaceVariant`.
No `Behavior` needed — it is static.

**Keyboard accelerators** (`&File` → `F` underline): noctalia has no accelerator rendering
convention. Strip the `_` / `&` prefix (already done via `.replace(/_/g, "")`) and do NOT
render the underline — it would be visually inconsistent with the rest of the shell.

**Toggle indicator** (`✓`, `•`): keep current approach but use `NText` at `Style.fontSizeS`
and `Color.mOnSurface` (not raw `Text` with `font.pixelSize`).

---

## 4. Layout

Outer popup padding (Column.anchors.margins):

```qml
anchors.margins: Style.marginS        // 6 px — matches NPopupContextMenu
```

Inter-row spacing: `Column.spacing: 0` (noctalia rows are edge-to-edge; visual gap comes
from the row height and the hover radius).

Text-to-edge gap: `Style.marginM` (9 px) left+right inside each row's `RowLayout`.

Minimum popup width: 180 px (current). Maximum width: unclamped; let `_recalcWidth` govern.

---

## 5. Animation

**Shell idiom** (`NPopupContextMenu.qml` lines 214-222):

```qml
// menuBackground opacity fade on open/close
opacity: root.visible ? 1.0 : 0.0
Behavior on opacity {
    NumberAnimation { duration: Style.animationNormal; easing.type: Easing.OutQuad }
}
```

`Style.animationNormal` defaults to 300 ms, respects `animationDisabled` and `animationSpeed`
user settings. `Easing.OutQuad` — not bounce, not elastic.

The plugin currently has no open/close animation. Add the same `opacity` + `Behavior` to
`menuBox` in both `AppmenuPopupWindow.qml` and `SubmenuPopup.qml`. Do **not** animate
width/height — layout thrash on a surface-resize is expensive on layer-shell surfaces.

---

## 6. Component inventory

| Slot | Current | Replace with | Source |
|---|---|---|---|
| Row label | raw `Text` | `NText` (pointSize: `Style.fontSizeS`) | `Widgets/NText.qml` |
| Toggle indicator | raw `Text` | `NText` | same |
| Shell icon | `NIcon` (already correct) | keep | `Widgets/NIcon.qml` |
| XDG app icon | `Image` (keep) | keep — NIcon is icon-font only | — |
| Separator | bare `Rectangle` | `NDivider` | `Widgets/NDivider.qml` |
| Hover background | `Rectangle` color=`Color.mHover` | correct token, add `ColorAnimation` Behavior | `NPopupContextMenu.qml:264` |
| Popup container | `Rectangle` color=`Color.mSurfaceVariant` | `Color.mSurface` | `Color.qml` |
| Border | `Color.mPrimary`, width 2 | `Color.mOutline`, `Style.borderS` | `NPopupContextMenu.qml:211` |
| Corner radius | `Style.marginS` (6 px) | `Style.radiusM` (16 px) | `Style.qml:33` |

`NText` and `NIcon` require `import qs.Widgets` to be added to `MenuRow.qml`.
`NDivider` requires `import qs.Widgets` (already uses Quickshell.Widgets internally).
