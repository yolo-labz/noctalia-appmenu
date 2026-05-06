//! `com.canonical.dbusmenu` client — fetches menu trees from
//! registered apps for the focus subsystem to mirror into the bar
//! widget.
//!
//! ## Wire format
//!
//! `GetLayout(parentId i32, recursionDepth i32, propertyNames as)`
//! returns `(revision u32, layout)`, where layout is the
//! externally-typed signature `(ia{sv}av)`:
//!
//! - `id: i32` — stable per-process menu item id
//! - `props: a{sv}` — string-keyed variant map. Standard keys:
//!   `label` (s, w/ `_` for accelerator marker),
//!   `type` (s, "" or "standard" or "separator" or "submenu"),
//!   `enabled` (b),
//!   `visible` (b),
//!   `icon-name` (s),
//!   `children-display` (s, "submenu" if has children),
//!   `accessible-desc` (s),
//!   `shortcut` (`aas`, list of "Ctrl"+"S" key combos),
//!   `toggle-type` (s, "checkmark" / "radio" / ""),
//!   `toggle-state` (i, 0=off 1=on -1=indeterminate),
//! - `children: av` — array of variants each containing a nested
//!   layout tuple. We unwrap them recursively.
//!
//! ## Why pull, not subscribe-and-cache
//!
//! Registered apps emit `LayoutUpdated(revision, parentId)` and
//! `ItemsPropertiesUpdated(updatedProps, removedProps)` signals on
//! tree mutation. v0.2 phase B does the simpler thing: re-fetch on
//! every focus change. Subscribe-and-cache is a v0.2.x
//! optimization once the basic flow is proven.
//!
//! ## Recursion-depth choice
//!
//! `GetLayout(0, -1, [])` asks for the FULL tree from root rooted
//! at id 0 with unbounded depth. Apps that respect the spec return
//! every nested item; some apps cap at 1 level (lazy-load via
//! AboutToShow). We start with `-1` and revisit if specific apps
//! need depth-1 + AboutToShow choreography.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zbus::{
    proxy,
    zvariant::{ObjectPath, OwnedValue},
    Connection,
};

/// `com.canonical.dbusmenu` proxy — minimal subset of methods we
/// currently invoke. Property accessors and signals live as future
/// work.
#[proxy(interface = "com.canonical.dbusmenu", default_path = "/MenuBar")]
trait DbusMenu {
    /// Returns `(revision, layout)`. The layout shape is described
    /// at the module top.
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> zbus::Result<(u32, MenuLayout)>;

    /// User clicked or hovered an item. We invoke this from the CLI
    /// click subcommand (Phase C of v0.2).
    #[allow(dead_code)]
    fn event(
        &self,
        id: i32,
        event_id: &str,
        data: zbus::zvariant::Value<'_>,
        timestamp: u32,
    ) -> zbus::Result<()>;
}

/// The opaque tuple shape `(ia{sv}av)` returned by `GetLayout`.
/// We deserialize via zbus zvariant types and then re-walk into
/// `MenuItem` for the JSON serialization downstream consumers use.
///
/// `Deserialize` + `Type` lets zbus' DBus wire decoder build this
/// directly from the `(ia{sv}av)` body. `OwnedValue` round-trip
/// derives let us bounce nested children through
/// `OwnedValue → MenuLayout` during the recursive parse_layout walk.
#[derive(
    Debug, Deserialize, zbus::zvariant::Type, zbus::zvariant::OwnedValue, zbus::zvariant::Value,
)]
pub struct MenuLayout {
    pub id: i32,
    pub props: HashMap<String, OwnedValue>,
    pub children: Vec<OwnedValue>,
}

/// Serializable representation of a single menu item + its full
/// subtree. This is what the QML widget reads from `active.json` to
/// render menus. Keeping the struct flat (no `Box<dyn>`) means
/// `serde_json::to_value` can roundtrip it without custom
/// serialisers.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MenuItem {
    /// `id` is the dbusmenu item id; the QML widget passes it back
    /// to the bridge's click subcommand to fire `Event(id,
    /// "clicked", …)` against the registered app.
    pub id: i32,
    pub label: String,
    /// `"standard"` (default), `"separator"`, or `"submenu"`.
    /// Apps sometimes omit this; we coerce missing → "standard".
    #[serde(rename = "type")]
    pub item_type: String,
    pub enabled: bool,
    pub visible: bool,
    /// `""` for items without an icon. We pass through the
    /// `icon-name` string directly; the QML side resolves via the
    /// system icon theme.
    pub icon_name: String,
    /// Empty when no toggle. Otherwise `"checkmark"` or `"radio"`.
    pub toggle_type: String,
    /// `0` off, `1` on, `-1` indeterminate. Only meaningful when
    /// `toggle_type` non-empty.
    pub toggle_state: i32,
    /// Empty when no children. Apps with `children-display:
    /// submenu` populate this on first `GetLayout` if recursion
    /// depth is `-1`.
    #[serde(default)]
    pub children: Vec<MenuItem>,
}

/// Helper: extract a string property from the prop map, falling
/// back to the given default. Walks the OwnedValue → Value →
/// borrowed `&str` chain because zvariant 4.x's `Str` variant
/// hides behind a Deref.
fn get_str(props: &HashMap<String, OwnedValue>, key: &str, default: &str) -> String {
    props
        .get(key)
        .and_then(|v| {
            let val: &zbus::zvariant::Value = v;
            <&str>::try_from(val).ok().map(str::to_owned)
        })
        .unwrap_or_else(|| default.to_string())
}

fn get_bool(props: &HashMap<String, OwnedValue>, key: &str, default: bool) -> bool {
    props
        .get(key)
        .and_then(|v| {
            let val: &zbus::zvariant::Value = v;
            bool::try_from(val).ok()
        })
        .unwrap_or(default)
}

fn get_i32(props: &HashMap<String, OwnedValue>, key: &str, default: i32) -> i32 {
    props
        .get(key)
        .and_then(|v| {
            let val: &zbus::zvariant::Value = v;
            i32::try_from(val).ok()
        })
        .unwrap_or(default)
}

/// Convert a wire `MenuLayout` into the serializable `MenuItem`
/// tree. Recursive but bounded by the depth the app actually
/// returned in `GetLayout`.
fn parse_layout(layout: MenuLayout) -> MenuItem {
    let label = get_str(&layout.props, "label", "");
    let item_type = get_str(&layout.props, "type", "standard");
    let enabled = get_bool(&layout.props, "enabled", true);
    let visible = get_bool(&layout.props, "visible", true);
    let icon_name = get_str(&layout.props, "icon-name", "");
    let toggle_type = get_str(&layout.props, "toggle-type", "");
    let toggle_state = get_i32(&layout.props, "toggle-state", 0);

    // Children are wrapped in `av` (array of variant). Each variant
    // contains another `(ia{sv}av)`. We unwrap one level at a time
    // by re-deserializing each child's OwnedValue back into
    // MenuLayout via try_from (zvariant 4.x's `OwnedValue::try_from`
    // dispatches on the embedded signature).
    let children = layout
        .children
        .into_iter()
        .filter_map(|owned| MenuLayout::try_from(owned).ok().map(parse_layout))
        .collect();

    MenuItem {
        id: layout.id,
        label,
        item_type,
        enabled,
        visible,
        icon_name,
        toggle_type,
        toggle_state,
        children,
    }
}

/// Fetch the full menu tree from a registered app.
///
/// `bus_name` is the unique connection name (e.g. `:1.42`) or the
/// well-known service name the app exported. `menu_path` is the
/// object path the app passed to `RegisterWindow`. Returns the
/// parsed root `MenuItem` (which itself has `children` —
/// the canonical root id 0 is a hidden anchor; its children are
/// the actual menubar tops).
pub async fn fetch_layout(
    conn: &Connection,
    bus_name: &str,
    menu_path: &ObjectPath<'_>,
) -> Result<MenuItem> {
    let proxy = DbusMenuProxy::builder(conn)
        .destination(bus_name)
        .with_context(|| format!("invalid destination {bus_name}"))?
        .path(menu_path.to_owned())
        .with_context(|| format!("invalid path {menu_path}"))?
        .build()
        .await
        .context("building dbusmenu proxy")?;

    let (_revision, layout) = proxy.get_layout(0, -1, vec![]).await.with_context(|| {
        format!("GetLayout({bus_name}, {menu_path}) — app may have left the bus")
    })?;

    Ok(parse_layout(layout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_item_default_is_empty() {
        let m = MenuItem::default();
        assert_eq!(m.id, 0);
        assert!(m.label.is_empty());
        assert!(m.children.is_empty());
    }

    #[test]
    fn menu_item_serializes_to_expected_json_shape() {
        let m = MenuItem {
            id: 1,
            label: "File".to_string(),
            item_type: "submenu".to_string(),
            enabled: true,
            visible: true,
            icon_name: "".to_string(),
            toggle_type: "".to_string(),
            toggle_state: 0,
            children: vec![MenuItem {
                id: 2,
                label: "Save".to_string(),
                item_type: "standard".to_string(),
                enabled: true,
                visible: true,
                ..Default::default()
            }],
        };
        let s = serde_json::to_string(&m).unwrap();
        // Spot-check critical keys, not the full string (key order
        // is implementation-defined on most serde_json versions).
        assert!(s.contains(r#""label":"File""#));
        assert!(s.contains(r#""type":"submenu""#));
        assert!(s.contains(r#""label":"Save""#));
        assert!(s.contains(r#""type":"standard""#));
    }

    #[test]
    fn menu_item_with_no_children_serializes_empty_array() {
        let m = MenuItem {
            id: 1,
            label: "Quit".to_string(),
            item_type: "standard".to_string(),
            enabled: true,
            visible: true,
            ..Default::default()
        };
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains(r#""children":[]"#));
    }
}
