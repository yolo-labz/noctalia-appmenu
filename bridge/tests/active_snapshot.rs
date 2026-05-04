//! Smoke test for ActiveSnapshot::empty() — verifies the public surface
//! exposed to the proxy module hasn't drifted.

use noctalia_appmenu_bridge::active::ActiveSnapshot;

#[test]
fn empty_snapshot_has_zero_pid_and_no_menu() {
    let s = ActiveSnapshot::empty();
    assert_eq!(s.focus_pid, 0);
    assert!(s.app_id.is_empty());
    assert!(s.title.is_empty());
    assert!(s.menu_service.is_empty());
    assert!(s.menu_path.is_none());
}
