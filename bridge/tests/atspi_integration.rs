//! Headless verifiers for the bridge's externally-observable contracts —
//! no live D-Bus, no desktop required. Two golden snapshots:
//!
//! 1. The serialized `MenuItem` model the plugin consumes via
//!    `active.json` — a Firefox-shaped menubar tree. Guards the wire
//!    shape (serde field names, the `type` rename, `#[serde(default)]`
//!    fields) the QML widget parses. Drift = the plugin silently
//!    mis-renders; CI fails first.
//! 2. The `org.noctalia.AppMenu.Active` D-Bus interface XML — the
//!    bridge↔plugin IPC contract. Drift without an ADR is exactly drift
//!    triggers G/I (a deploy whose surface changed under the plugin).
//!
//! Per the repo's "zbus has no mock crate" rule, both mock at the MODEL
//! layer with insta — never the bus layer. The live-bus integration
//! lives in the `just integration` lane (niri-headless + AT-SPI fixture);
//! the fake-registrar README points here for the model-layer half.

use noctalia_appmenu_bridge::atspi::MenuItem;

/// A top-level menu (File, Edit, …) as produced by a menubar walk before
/// its children are realised. Firefox lazy-realises submenu items on
/// first open (the self-heal cascade fills them later), so a freshly
/// walked menubar has empty `children` on each top-level entry.
fn top_menu(id: i32, label: &str) -> MenuItem {
    MenuItem {
        id,
        label: label.to_string(),
        item_type: "submenu".to_string(),
        enabled: true,
        visible: true,
        icon_name: String::new(),
        toggle_type: String::new(),
        toggle_state: 0,
        service: ":1.84".to_string(),
        path: format!("/org/a11y/atspi/accessible/{}", 100 + id),
        children: Vec::new(),
    }
}

/// Golden: the serialized `MenuItem` tree for a Firefox-shaped menubar
/// (root "Application" → File/Edit/View/History/Bookmarks/Profiles/
/// Tools/Help — the exact shape verified live on niri at v1.0.29). This
/// is the JSON the bridge writes into `active.json` and the QML plugin
/// parses. If serde output drifts — a renamed field, a dropped
/// `#[serde(default)]`, the `type` rename lost — the snapshot fails and
/// the change must be intentional and reviewed before it can ship.
#[test]
fn firefox_menubar_serializes_to_stable_json() {
    let labels = [
        "File",
        "Edit",
        "View",
        "History",
        "Bookmarks",
        "Profiles",
        "Tools",
        "Help",
    ];
    let menu = MenuItem {
        id: 0,
        label: "Application".to_string(),
        item_type: "submenu".to_string(),
        enabled: true,
        visible: true,
        icon_name: String::new(),
        toggle_type: String::new(),
        toggle_state: 0,
        service: ":1.84".to_string(),
        path: "/org/a11y/atspi/accessible/root".to_string(),
        children: labels
            .iter()
            .enumerate()
            .map(|(i, l)| top_menu(i as i32 + 1, l))
            .collect(),
    };

    insta::assert_json_snapshot!(menu);
}

/// Golden: the introspection XML of the `org.noctalia.AppMenu.Active`
/// interface the plugin binds to. Snapshotted headlessly via zbus's
/// `Interface::introspect_to_writer` — no bus, no connection. A drift
/// means the bridge↔plugin IPC surface changed; CI fails until the
/// golden is updated *with* an ADR explaining the contract change
/// (governance triggers G/I — the running plugin reads this interface).
#[test]
fn active_proxy_interface_xml_is_stable() {
    use noctalia_appmenu_bridge::proxy::ActiveProxy;
    use zbus::object_server::Interface;

    let proxy = ActiveProxy::new();
    let mut xml = String::new();
    proxy.introspect_to_writer(&mut xml, 0);

    insta::assert_snapshot!(xml);
}
