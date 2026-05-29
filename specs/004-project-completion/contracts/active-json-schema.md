# Contract: `active.json` schema (consumer-facing)

**Status:** v=1 since PR #59; extended at v1.0.0 with a `source` field
**Path:** `~/.cache/noctalia-appmenu/active.json`
**Producer:** `bridge/src/active.rs`
**Consumers:** `plugin/BarWidget.qml` via `FileView` + `IpcHandler` push for steady-state

## Schema (v=1.1, v1.0.0)

```json
{
  "v": 1,
  "pid": 12345,
  "app_id": "org.kde.kate",
  "title": "Document — kate",
  "source": "atspi",
  "menu": {
    "id": 0,
    "label": "",
    "item_type": "submenu",
    "enabled": true,
    "visible": true,
    "icon_name": "",
    "toggle_type": null,
    "toggle_state": null,
    "service": ":1.123",
    "path": "/org/a11y/atspi/accessible/root",
    "children": [ ...MenuItem ]
  }
}
```

## Field semantics

| Field | Type | Required | Notes |
|---|---|---|---|
| `v` | integer `1` | yes | Schema version. Consumers reject snapshots with `v != 1`. |
| `pid` | integer ≥ 0 | yes | Focused process PID. `0` is reserved for "no focus" — paired with `source = "empty"`. |
| `app_id` | string | yes | Reverse-DNS app identifier as resolved by the focus sink. Empty when `source = "empty"`. |
| `title` | string | yes | Window title; may be empty. |
| `source` | string enum | **new at v1.0.0** | One of `"atspi"`, `"synthetic"` (legacy, unused), `"desktop-fallback"` (spec 016 / ADR-0031), `"empty"`. |
| `menu` | object (`MenuItem` root) | yes | Walker output or synthetic fallback. `children: []` is mandatory when `source = "empty"`. |

## `MenuItem` shape

| Field | Type | Notes |
|---|---|---|
| `id` | integer | Walker-assigned monotonic; not stable across walks. |
| `label` | string | Accessible Name with accelerator markers stripped. |
| `item_type` | string enum | `"standard"`, `"separator"`, `"submenu"`. |
| `enabled` | bool | AT-SPI states `ENABLED` ∧ `SENSITIVE`. |
| `visible` | bool | AT-SPI states `VISIBLE` ∧ `SHOWING`. |
| `icon_name` | string | freedesktop icon-theme name; empty when absent. |
| `toggle_type` | string enum or null | `"checkmark"`, `"radio"`, or null. |
| `toggle_state` | bool or null | Defined iff `toggle_type` is non-null. |
| `service` | string | AT-SPI bus connection name (e.g. `":1.123"`). |
| `path` | string | AT-SPI object path (e.g. `"/org/a11y/atspi/accessible/42"`). |
| `children` | array of `MenuItem` | DFS subtree. Empty for `standard` leaves. |

## Wire compatibility

- Adding a new top-level field is non-breaking when its absence is treated as `null` / default by the consumer. The `source` field is added at v1.0.0; consumers that ignore it continue to function.
- Renaming or removing any v=1 field is **breaking**: requires `v=2` and lockstep update of the plugin reader.

## Validation rules

1. `pid >= 0`.
2. `pid == 0` ⇒ `source == "empty"`.
3. `source == "empty"` ⇒ `menu` is `null` (the producer writes `null`, not an empty `MenuItem`; consumers treat both as "no menu").
4. `source == "synthetic"` ⇒ `app_id` is non-empty. (Legacy; no live producer — superseded by `desktop-fallback`.)
5. `source == "desktop-fallback"` ⇒ `app_id` is non-empty AND `menu` is a non-null `MenuItem` with ≥ 1 child. Built only after AT-SPI returns no menubar, so it never co-occurs with `source == "atspi"` (spec 016 / ADR-0031).
6. `toggle_state` is null iff `toggle_type` is null.
7. For each `MenuItem` in `menu.children` (recursively): `item_type == "submenu"` iff `children` is non-empty.

## Producer-side dedup contract

Per PR #59 (active.json schema v=1 + producer-side dedup): the producer MUST NOT re-emit a snapshot whose serialised bytes are identical to the previous one. This is the consumer's only guarantee that `FileView`-triggered re-renders correspond to real state changes.

## Test contract

- A producer-side unit test verifies that two identical snapshots in succession produce exactly one file write.
- A consumer-side QML test loads three reference snapshots from `tests/fixtures/` (atspi, synthetic, empty) and asserts the BarWidget renders the expected number of strip items, popup rows, and check indicators.

## Non-goals

- The schema does **not** carry compositor identification. The bridge is the only producer; consumers do not need to know which compositor sourced the focus event.
- The schema does **not** carry the user's locale; label translation is the app's responsibility (already-translated `label` strings flow through unchanged).
