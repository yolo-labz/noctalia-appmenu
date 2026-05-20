# Visual-parity audit — spec 015 SC-002

Snapshot at commit `ce562f2` (`origin/main` HEAD when this audit was
generated). Source checklist: `checklists/visual-parity.md`. The
release gate `scripts/verify-release.sh` (spec 015 FR-007) re-runs
this audit per release; the gate fails on any FAIL row.

Methodology — each row is the literal grep used to derive PASS/FAIL,
paired with the matched line(s). Reproducible by running the same
grep against the working tree.

## Surface treatment

| ID | Status | Evidence |
|----|--------|----------|
| VP-001 | PASS | `plugin/AppmenuPopupWindow.qml:217` — `color: Color.mSurface` |
| VP-002 | PASS | `grep border.color plugin/AppmenuPopupWindow.qml` → zero hits |
| VP-003 | PASS | `plugin/AppmenuPopupWindow.qml:226` — `border.width: 0` |
| VP-004 | PASS | `plugin/AppmenuPopupWindow.qml:222-225` — `topLeftRadius: 0`, `topRightRadius: 0`, `bottomLeftRadius/bottomRightRadius: Style.radiusL` |
| VP-005 | PASS | `plugin/SubmenuPopup.qml:176` — symmetric `radius: Style.radiusL` |
| VP-006 | PASS | `plugin/AppmenuPopupWindow.qml:185` — sibling `NDropShadow` gated by `Settings.data.general.enableShadows` (not `layer.effect`) |

## Row treatment

| ID | Status | Evidence |
|----|--------|----------|
| VP-010 | PASS | `plugin/MenuRow.qml:66-68` — `height: isSeparator ? Style.marginM : 28`. Literal `28` is the documented match for `NPopupContextMenu.qml:248`; visual-spec §2 sanctions it. |
| VP-011 | PASS | `plugin/MenuRow.qml:100` — `Color.mHover !== undefined ? Color.mHover : Color.mSurfaceVariant` (defensive fallback, mHover preferred) |
| VP-012 | PASS | `plugin/MenuRow.qml:105-107` — `Behavior on color { ColorAnimation { duration: Style.animationFast !== undefined ? Style.animationFast : 150 } }` |
| VP-013 | PASS | `plugin/MenuRow.qml:102` — `radius: Style.iRadiusXS !== undefined ? Style.iRadiusXS : 8` |
| VP-014 | PASS | `plugin/MenuRow.qml:103` — `opacity: row.isEnabled ? 1.0 : 0.5` |

## Layout

| ID | Status | Evidence |
|----|--------|----------|
| VP-020 | PASS | `plugin/AppmenuPopupWindow.qml:216,254` — outer container sized via `Style.marginM` |
| VP-021 | PASS | `plugin/MenuRow.qml:83,84,113,114` — `anchors.{left,right}Margin: Style.marginM` |
| VP-022 | PASS | `plugin/AppmenuPopupWindow.qml:255` — `spacing: 0` |
| VP-023 | PASS | `plugin/AppmenuPopupWindow.qml:215` — `width: Math.max(220, root._calcWidth)` |
| VP-024 | PASS | `plugin/AppmenuPopupWindow.qml:136..157` — `_calcWidth` driver, no fixed max |

## Typography

| ID | Status | Evidence |
|----|--------|----------|
| VP-030 | PASS | `plugin/MenuRow.qml:118,163,180` — `NText` (not raw Text) |
| VP-031 | PASS | `plugin/MenuRow.qml:130,169,184` — `pointSize: Style.fontSizeS` |
| VP-032 | PASS | `plugin/MenuRow.qml:167-168` — `containsMouse ? Color.mOnHover : Color.mOnSurface`; `Behavior on color` at 172 |
| VP-033 | PASS | `plugin/MenuRow.qml:118-131` — toggle indicator is `NText`, U+2713/U+2022 |
| VP-034 | PASS | `plugin/MenuRow.qml:180-185` — chevron is `NText` U+203A, color `Color.mOnSurfaceVariant`, no Behavior (static) |
| VP-035 | PASS | `plugin/{AppmenuPopupWindow,BarWidget,MenuRow,SubmenuPopup}.qml` — `.replace(/_/g, "")` at all label render sites |

## Separators

| ID | Status | Evidence |
|----|--------|----------|
| VP-040 | PASS | `plugin/MenuRow.qml:78` — `NDivider` |
| VP-041 | PASS | `plugin/MenuRow.qml:83-84` — leftMargin / rightMargin: `Style.marginM` |

## Animation

| ID | Status | Evidence |
|----|--------|----------|
| VP-050 | PASS | `plugin/AppmenuPopupWindow.qml:231-234` — `Behavior on opacity { NumberAnimation { duration: Style.animationNormal; easing.type: Easing.OutQuad } }` |
| VP-051 | PASS | `grep -nE 'Behavior on (width\|height\|implicitHeight)' plugin/{AppmenuPopupWindow,MenuRow,SubmenuPopup}.qml` → zero hits |
| VP-052 | PASS | `grep -nE 'OutBounce\|InBounce\|OutElastic\|InBack\|cubic-bezier' plugin/*.qml` → zero hits |

## Token discipline

| ID | Status | Evidence |
|----|--------|----------|
| VP-060 | PASS | `grep -nE '#[0-9a-fA-F]{6}' plugin/*.qml \| grep -v '^[^:]*:[0-9]+:\s*//'` → zero hits |
| VP-061 | PASS | `grep -nE 'font\.pixelSize\s*:\s*[0-9]' plugin/*.qml \| grep -v '_measureText'` → zero hits |
| VP-062 | PASS | `grep -nE '(^\|[^.])\bradius\s*:\s*[0-9]' plugin/*.qml \| grep -v '^[^:]*:[0-9]+:\s*//'` → zero hits |
| VP-063 | PASS | `grep -nE 'border\.width\s*:\s*[1-9]' plugin/*.qml \| grep -v '^[^:]*:[0-9]+:\s*//'` → zero hits |
| VP-064 | PASS (this PR) | Previous state: FAIL — `plugin/BarWidget.qml:452,455` used `duration: 180` literal. Fixed in this PR to `Style.animationNormal !== undefined ? Style.animationNormal : 180` (defensive fallback, matches `MenuRow.qml:107`). |

## Roll-up

23 rows. Zero FAIL after the in-PR fix to VP-064. Gate `visual-smoke`
(spec 015 FR-007) emits `gates/visual.sh::PASS` for v1.0.23.

## Reproducer

```bash
# From repo root, on a clean working tree:
bash specs/015-ship-ready-completion/gates/visual.sh

# Or invoke verify-tokens directly:
bash scripts/verify-tokens.sh plugin/*.qml
```

## Maintenance

- Add a new VP-NNN row to `checklists/visual-parity.md` ONLY when the
  visual-spec §2 changes; this audit re-derives from that source.
- The pre-commit hook `verify-tokens` (lefthook) blocks new VP-060..VP-064
  violations on staged files — pre-existing exceptions must be added to
  this audit with an explicit rationale, not allow-listed in the script.
