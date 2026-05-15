# Contract: Qt6 menu wrapper recursive flatten

Spec: `specs/009-popup-hotfix/spec.md` FR-001

## Producer

`bridge/src/atspi.rs::fetch_menu_tree` (and any internal helper it
calls during recursive descent).

## Consumer

`bridge/src/active.rs::write_active_snapshot` consumes the flattened
tree via the in-memory `AtspiMenuItem` and serialises it to
`active.json`. QML's `BarWidget.qml` then renders.

## Behavioural contract

For every `AtspiMenuItem` returned by `fetch_menu_tree` at any
depth, the following invariant MUST hold:

> NOT (item.children.len() == 1
>      AND item.children[0].label.is_empty()
>      AND item.children[0].type_ == AtspiItemType::Submenu)

I.e. the tree contains zero "empty-label MENU wrappers". The
flatten happens IN PLACE inside `fetch_menu_tree` immediately after
each child is fetched (bottom-up), so the invariant holds for the
entire returned tree by induction on depth.

## Test contracts

- **Fixture.** `bridge/tests/fixtures/qt_nested_wrapper.json` —
  static JSON snapshot of an AT-SPI tree with the wrapper at three
  levels (mirrors shadPS4QtLauncher's `View > Game List Mode >
  [List, Grid, Flat]` shape).
- **Round-trip test.** `bridge/tests/atspi_flatten.rs` (new file)
  loads the fixture, deserialises into a mock AT-SPI tree
  representation, runs the flatten algorithm, asserts:
  - Top-level item has `children.len() == 3` (not 1).
  - Top-level child labelled "Game List Mode" has
    `children == [List, Grid, Flat]` (not a single empty-label
    wrapper).
  - No item at any depth in the returned tree satisfies the wrapper
    pattern.
- **Performance.** Same test asserts the flatten completes in
  under 100µs on the fixture (NFR-003 budget guard; FETCH_BUDGET
  is 3000ms which gives 30000× headroom — this assertion is a
  drift detector, not a tight bound).

## Edge cases

- **Multi-child wrapper.** A `MENU_ITEM` with
  `children = [{label: "", type: MENU, children: [...]}, {label:
  "Other", ...}]` is NOT a wrapper case (multiple children means
  Qt is using the label-less child intentionally). MUST NOT
  flatten. Test contract: fixture variant with this shape; assert
  zero flattens.
- **Wrapper inside wrapper.** Theoretically Qt could emit
  `MENU_ITEM → MENU(empty) → MENU(empty) → [items]`. The recursive
  bottom-up flatten handles this naturally — innermost wrapper
  flattens first, then middle, then no-op at top. Test contract:
  fixture variant; assert one flatten removes both wrappers.
- **Empty leaf.** A `MENU_ITEM` with `children = []` and no label
  is a leaf separator-like item — NOT a wrapper case. MUST NOT
  flatten (no `children[0]` to flatten anyway).
- **Toggle / radio items.** Items with `toggle_state` set MUST
  preserve `toggle_state` and `type_` after flatten — flatten
  applies to the wrapping MENU, not to the leaf items.
