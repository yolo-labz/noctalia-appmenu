# ADR-0023 — fetch DBusMenu trees on focus change

- **Status:** Accepted (2026-05-05)
- **PR:** #30
- **Released in:** v0.2.0-alpha (Phase B+C of v0.2 epic)

## Context

ADR-0022 (PR #29) made the bridge own
`com.canonical.AppMenu.Registrar` so apps can register their menu
paths against us. With v0.1 plumbing, the focus subsystem now sees
populated `(busName, menuPath)` pairs for any registered focused
app — but we still write only the path coordinates to
`active.json`, never the actual menu tree. The QML widget can't
render a menu strip without that data.

## Decision

The `proxy.rs` write loop fetches the focused app's full DBusMenu
tree on every snapshot change and serializes it as a nested JSON
field (`menu: { id, label, type, enabled, visible, children: [...]
}`) inside `active.json`.

Implementation:

1. **New module `bridge/src/dbusmenu.rs`** — `com.canonical.dbusmenu`
   client proxy. Single public entry point: `fetch_layout(conn,
   busName, menuPath) -> Result<MenuItem>`. Internally calls
   `GetLayout(0, -1, [])` (root id, unbounded depth, no property
   filter) to retrieve the entire menu tree in one round-trip. The
   wire shape `(ia{sv}av)` is consumed via a private
   `MenuLayout` struct deriving zbus `Type + Deserialize + Value +
   OwnedValue`. Each child variant is recursively re-decoded into
   `MenuLayout` via `OwnedValue::try_into` so the recursion never
   has to hand-walk a `serde_json::Value`-style untyped tree.

2. **Public `MenuItem` struct** — flat-recursive serializable
   shape: `{id, label, type, enabled, visible, icon_name,
   toggle_type, toggle_state, children: [...]}`. Field names are
   the snake_case identifiers QML reads from JSON. `type` is
   serialized via `#[serde(rename = "type")]`.

3. **Property extraction helpers** — `get_str`, `get_bool`,
   `get_i32` walk `OwnedValue → &Value → primitive` once per
   property, with sane defaults (label="", type="standard",
   enabled=true, visible=true). Handles app variation gracefully:
   apps that omit `type` get treated as standard items, etc.

4. **`proxy.rs` integration** — between the existing
   `borrow_and_update()` and `write_active_json()`, the loop calls
   `dbusmenu::fetch_layout(...)` if the snapshot has both
   `menu_service != ""` AND `menu_path = Some(_)`. Errors are
   non-fatal: log a warn, write `menu: null` to `active.json`, and
   the QML widget falls back to its v0.1 placeholder.

5. **Recursion-depth choice** — `GetLayout(0, -1, …)` asks for the
   FULL tree. Apps that respect the spec (gimp, inkscape, qbittorrent)
   return everything. Some apps cap at depth 1 and expect
   `AboutToShow` choreography for submenus. We start with the
   simple full-fetch; AboutToShow lazy-load is a v0.2.x
   optimization.

## Consequences

- **`active.json` schema gains a `menu` field.** v0.1 widgets that
  don't read `menu` are unaffected (JSON is additive). v0.2 widgets
  read `j.menu.children` to render the top-level menu strip.
- **Per-focus-change network round-trip to the focused app.**
  P95 latency budget is ~5ms over loopback D-Bus per spec 002
  NFR-001. Apps with deep menus (Anki has ~8 top-level × 6 nested)
  serialize ~50 layout tuples; well under the budget.
- **Apps disappearing mid-fetch don't crash the bridge.** zbus
  `proxy.get_layout()` returns `Err` when the destination is gone;
  we log + continue. This handles the user-closes-window race.
- **No subscribe-to-LayoutUpdated yet.** If an app mutates its menu
  while focused (e.g. kate opens a buffer, "Window" menu adds a
  new entry), the bar won't reflect that until the next focus
  change. Acceptable for v0.2.0-alpha; `LayoutUpdated`/
  `ItemsPropertiesUpdated` subscription is deferred to v0.2.1.

## Alternatives considered

- **Subscribe to `LayoutUpdated` + cache layouts per-app:** correct
  but invites cache-staleness bugs. The simple per-focus refetch is
  ~50 D-Bus calls × few ms each in the worst case; user-perceived
  latency is dominated by the focus event itself, not the menu
  fetch. Optimize when load is measurable.
- **Fetch only top-level + lazy-load submenus on hover:** correct
  for app correctness with `AboutToShow` semantics, but lazy-load
  introduces visible-pause UX on first submenu open. Full eager
  fetch keeps interaction snappy at the cost of slightly more
  D-Bus traffic on focus.
- **Stream the menu tree over a Unix socket instead of JSON file:**
  considered but rejected — FileView watch+reload is already wired
  in the QML widget (v0.1.9), and per-focus-change writes are
  cheap. Socket adds protocol surface for no gain at this size.

## Wire format reference

`com.canonical.dbusmenu::GetLayout(parentId i32, recursionDepth
i32, propertyNames as) -> (revision u32, layout (ia{sv}av))`:

- `id: i32` — stable per-process menu item id.
- `props: a{sv}` — string-keyed variant map. Standard keys read by
  this implementation:
  - `label` — `s` (with `_` for accelerator marker)
  - `type` — `s` (`""` / `"standard"` / `"separator"` / `"submenu"`)
  - `enabled` — `b`
  - `visible` — `b`
  - `icon-name` — `s`
  - `toggle-type` — `s` (`"checkmark"` / `"radio"` / `""`)
  - `toggle-state` — `i` (0/1/-1)
- `children: av` — array of variants, each containing a nested
  `(ia{sv}av)`. Recursion bottoms out at items with empty
  `children` array.

Spec:
https://github.com/AyatanaIndicators/libdbusmenu/blob/master/libdbusmenu-glib/com.canonical.dbusmenu.xml

## References

- ADR-0022 (registrar server side)
- Spec 002 (DBusMenu mirror) — Phase B + C delivered here
- Phase D (widget render) → PR #31
- v0.2.0-alpha release notes
