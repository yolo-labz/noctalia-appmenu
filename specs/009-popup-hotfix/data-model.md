# Data model — spec 009-popup-hotfix

Spec: `specs/009-popup-hotfix/spec.md`

This hotfix is mostly behavioural, not data-model-changing. Only one
schema field is added (FR-006), and it is OPTIONAL + additive.

---

## Entity: `ActiveSnapshot` (extends ADR-0024 schema, additive only)

The bridge writes a JSON object to
`$XDG_CACHE_HOME/noctalia-appmenu/active.json` on every focus event.
Existing schema (v1.0.0):

```json
{
  "v": 1,
  "app_id": "string",
  "focus_pid": "integer",
  "title": "string",
  "menu_service": "string",
  "menu_path": "string",
  "menu": null | { "id": "...", "label": "...", "type": "...",
                   "enabled": bool, "visible": bool,
                   "children": [ ...recursive ] }
}
```

### Field added (FR-006): `focused_output`

| Field | Type | Required? | Default if absent | Source |
|---|---|---|---|---|
| `focused_output` | string \| null | NO (additive) | `null` | niri-IPC focus event payload |

**Validation rule.** When present, `focused_output` MUST be a non-
empty string matching the `name` of one of the connected wayland
outputs (e.g. `"DP-1"`, `"HDMI-A-1"`, `"eDP-1"`). When the focused
window has no output binding (extremely rare on niri — usually only
during compositor restart), the field MUST be `null`, NOT an empty
string.

**Backward compatibility.** Consumers MUST tolerate the field's
absence. The QML `BarWidget.focusedScreenName` derivation already
defaults to `""` when `Quickshell.Wayland.ToplevelManager` returns
no data; with this addition it consults `active.json.focused_output`
between the Quickshell try and the empty fallback.

### Field NOT added: `menu_generation`

Considered as a cleaner alternative to FR-005's children-shape dedup
proxy. Rejected because it requires bridge-side state (counter
maintenance + invalidation rules) and the QML-side proxy works for
the actual failure mode. Revisit if FR-005's proxy ever produces
false negatives.

---

## Entity: `AtspiMenuItem` (in-memory, bridge-only)

Internal struct used by `bridge/src/atspi.rs::fetch_menu_tree`.
Existing definition (per ADR-0024):

```rust
pub struct AtspiMenuItem {
    pub id: String,
    pub label: String,
    pub type_: AtspiItemType,  // Action / Submenu / Separator / Toggle
    pub enabled: bool,
    pub visible: bool,
    pub icon_name: Option<String>,
    pub toggle_state: Option<bool>,
    pub service: String,    // a11y bus name
    pub path: String,       // a11y object path
    pub children: Vec<AtspiMenuItem>,
}
```

### Validation rule added (FR-001)

After `fetch_menu_tree` returns for any item, the following
invariant MUST hold for every item in the tree:

> If `item.children.len() == 1` AND
> `item.children[0].label.is_empty()` AND
> `item.children[0].type_ == Submenu`,
> THEN that single empty-label child MUST be flattened away
> (its grandchildren become `item.children`) BEFORE the item is
> returned to the caller.

Test contract: `bridge/tests/atspi_flatten.rs` feeds a fixture
mirroring shadPS4QtLauncher's `View > Game List Mode > [List, Grid,
Flat]` accessibility tree (with the wrapper at every level) and
asserts the post-flatten tree contains zero empty-label MENU
intermediates at any depth.

---

## Entity: `MenuItemNode` (QML-side)

JS object dictated by `active.json.menu` shape. Same schema as
above; rendered by `BarWidget.qml`'s `Repeater` and forwarded to
`AppmenuPopupWindow` / `SubmenuPopup` / `MenuRow` delegates.

### Behavioural invariant added (FR-005)

`BarWidget.qml::_sameTopLevel(a, b)` MUST return `false` when:
- `a.length !== b.length`, OR
- any `a[i].id`, `a[i].label`, `a[i].enabled` differs from `b[i].*`,
  OR
- any `a[i].children.length !== b[i].children.length`, OR
- any `a[i].children[j].label !== b[i].children[j].label` for the
  first level of children.

Returning `true` means the model is reused; returning `false` means
`root.topLevel = b` is assigned and the Repeater rebuilds.

---

## State transitions

### `BarWidget._failedState` (FR-008 widening)

| From | Event | To | Notes |
|---|---|---|---|
| `false` | `_applySnapshotInner` throws | `true` | Existing behaviour; widget hides via `shouldRender = false`. |
| `true` | `_applySnapshotInner` succeeds | `false` | Widened to clear on ANY successful apply, even when the snapshot is structurally identical to the one that caused the latch (currently the dedup short-circuits the success path). |
| `true` | `applySnapshot(null)` (clear) | `false` | New transition — explicit clears MUST also drop the latch. |

### `Loader.status` (FR-004 wait)

| From | Event | Action |
|---|---|---|
| `Null` (initial) | `sourceComponent = nestedComponent` | Begin async instantiate. |
| `Loading` | (engine internal tick) | Status changes to `Ready` or `Error`. |
| `Ready` | (delivered to handler) | `nestedLoader.item.open(item, anchor)` fires. |
| `Error` | (component failed) | Log + return; do not re-attempt this open. |

### Submenu cascade depth (FR-007 namespace)

| Depth | Surface | Namespace |
|---|---|---|
| 0 | `AppmenuPopupWindow` | `noctalia-appmenu-popup-<screen>` |
| 1 | First `SubmenuPopup` | `noctalia-appmenu-submenu-d1-<screen>` |
| 2 | Second-level `SubmenuPopup` (recursive) | `noctalia-appmenu-submenu-d2-<screen>` |
| N | Nth-level `SubmenuPopup` | `noctalia-appmenu-submenu-dN-<screen>` |

Depth is bounded in practice by the AT-SPI menu tree shape (≤4 in
the wild per ADR-0024 §Failure-modes; shadPS4QtLauncher's deepest
is 3).
