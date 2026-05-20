# Visual-parity checklist — spec 015 SC-002 gate

**Owner:** `scripts/verify-release.sh` gate `visual-smoke`
**Source of truth:** `specs/013-sota-overhaul/visual-spec.md`
+ live shell snapshot at
`/nix/store/k7ylwjx92y0lbch5gydbx17mjiy6vblz-noctalia-share-patched/`

Each row is a finite pass/fail check. The release gate runs
`grep -c '^.*FAIL.*$' visual-audit.md`; a non-zero count aborts
the release.

## Surface treatment

- [ ] **VP-001** Popup container background == `Color.mSurface`.
      *Check:* `grep -n 'color: Color\.' plugin/AppmenuPopupWindow.qml`
      shows `Color.mSurface` for the menu Rectangle. No literal hex.
- [ ] **VP-002** No `border.color` set on the menuBox (visible
      stroke off — edge defined by radius + shadow only).
      *Check:* `grep -n 'border\.color' plugin/AppmenuPopupWindow.qml`
      returns zero hits.
- [ ] **VP-003** `border.width: 0` on menuBox (explicit).
- [ ] **VP-004** Top-attach corners: `topLeftRadius: 0`,
      `topRightRadius: 0`, `bottomLeftRadius: Style.radiusL`,
      `bottomRightRadius: Style.radiusL` on the bar-attached popup.
- [ ] **VP-005** Submenu cascade keeps symmetric `radius: Style.radiusL`
      (no asymmetric corners on submenu — it does not attach to the
      bar).
- [ ] **VP-006** Drop shadow rendered via sibling NDropShadow item
      with `source: menuBox`, gated by
      `Settings.data.general.enableShadows`. Not a `layer.effect`.

## Row treatment

- [ ] **VP-010** Row height fixed at `28` pixels (matches
      NPopupContextMenu.qml:248 + visual-spec §2).
- [ ] **VP-011** Row hover color == `Color.mHover` (NOT
      `Color.mSurfaceVariant`).
- [ ] **VP-012** Row hover transition: `Behavior on color { ColorAnimation
      { duration: Style.animationFast } }`. Duration token, not literal.
- [ ] **VP-013** Row corner radius == `Style.iRadiusXS` per
      visual-spec §2. No literal `4` or `6`.
- [ ] **VP-014** Disabled row opacity `0.5` (shell idiom — wrapping
      surface). NText inner opacity stays at `enabled ? 1.0 : 0.6`.

## Layout

- [ ] **VP-020** Popup outer padding (Column.anchors.margins) ==
      `Style.marginM` (per spec 015 FR-004 — bumped from marginS).
- [ ] **VP-021** Row left/right margin == `Style.marginM` inside RowLayout.
- [ ] **VP-022** Inter-row spacing `Column.spacing: 0` (edge-to-edge
      rows — visual gap from row height + hover radius).
- [ ] **VP-023** Minimum popup width == `220 px`.
- [ ] **VP-024** Width clamp uses `_recalcWidth`; no fixed max width.

## Typography

- [ ] **VP-030** Row label is `NText` (not raw `Text`). Imports
      `qs.Widgets`.
- [ ] **VP-031** `pointSize: Style.fontSizeS`. No literal pixelSize.
- [ ] **VP-032** Color: `rowHover.containsMouse ? Color.mOnHover :
      Color.mOnSurface`. Animated via `Behavior on color
      { ColorAnimation { duration: Style.animationFast } }`.
- [ ] **VP-033** Toggle indicator (`✓`, `•`) uses NText, not raw Text.
- [ ] **VP-034** Submenu chevron is the `›` (U+203A) character at
      `Style.fontSizeS`, color `Color.mOnSurfaceVariant`. Static —
      no Behavior.
- [ ] **VP-035** Keyboard accelerator underscores stripped via
      `.replace(/_/g, "")`. No underline rendered.

## Separators

- [ ] **VP-040** Separator delegate uses `NDivider`, not a bare
      Rectangle.
- [ ] **VP-041** NDivider has `anchors.leftMargin: Style.marginM`,
      `anchors.rightMargin: Style.marginM`.

## Animation

- [ ] **VP-050** Popup open/close: `opacity: visible ? 1.0 : 0.0`
      with `Behavior on opacity { NumberAnimation { duration:
      Style.animationNormal; easing.type: Easing.OutQuad } }`.
- [ ] **VP-051** No animation on width/height (layer-shell
      surface-resize is expensive).
- [ ] **VP-052** No bounce/elastic/back easing anywhere in popup
      QML — visual-spec §5 hard ban.

## Token discipline

- [ ] **VP-060** No raw hex literal in `plugin/*.qml` outside header
      comments. *Check:* `grep -nE '#[0-9a-fA-F]{6}' plugin/*.qml |
      grep -v '^[^:]*:[0-9]*:\s*//'` returns zero hits.
- [ ] **VP-061** No `font.pixelSize: <number>` outside the
      bar-strip measurement widget (`_measureText` is allowed —
      sizes against the bar's own font scale).
- [ ] **VP-062** No `radius: <number>` (must reference `Style.radius*`).
- [ ] **VP-063** No `border.width: <number>` (must reference
      `Style.borderS` or be `0`).
- [ ] **VP-064** No `duration: <number>` (must reference
      `Style.animation*`).

## Result roll-up

When this checklist is rendered into `visual-audit.md` via the
verify-release script, each row carries a literal `PASS` or
`FAIL` column. The gate fails on any FAIL.
