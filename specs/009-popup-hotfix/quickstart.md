# Quickstart — spec 009-popup-hotfix

Spec: `specs/009-popup-hotfix/spec.md`
Plan: `specs/009-popup-hotfix/plan.md`

## What you're building

A pair of focused PRs that fix v1.0.0's submenu render and click-
drop regressions on Qt6 apps. One PR per lane:

- **Lane Q (QML)** — popup geometry, cascade async-safety, dedup
  widening, namespace uniqueness, failed-state self-clear, screen
  fallback consume-side.
- **Lane B (Bridge)** — recursive Qt menu wrapper-flatten, screen
  fallback produce-side, version bump to 1.0.1.

## Local setup

```bash
# You should already be in the worktree:
cd ~/Documents/Code/yolo-labz/noctalia-appmenu-009-popup-hotfix
git status              # clean, on 009-popup-hotfix tracking origin/main

# Enter dev shell:
nix develop
```

## Lane Q — implementation walk-through

Files to edit:
- `plugin/AppmenuPopupWindow.qml`
- `plugin/SubmenuPopup.qml`
- `plugin/BarWidget.qml`

Files to add:
- `plugin/tests/qmltest/popup_geometry.qml`
- `plugin/tests/qmltest/submenu_cascade.qml`

Step-by-step:

1. **`AppmenuPopupWindow.qml`** — drop `anchors.bottom: true` and
   `anchors.right: true` (keep top + left for screen origin).
   Add `width: menuBox.width` and `height: menuBox.height`.
   Replace the `mapToItem(null, 0, 0)` calls with
   `mapToGlobal(0, 0)` for `x`/`y` derivation. Remove the
   full-screen outside-click `MouseArea`.
2. **`SubmenuPopup.qml`** — same surface fixes as #1. Add
   `property int depth: 1` and bind `WlrLayershell.namespace` to
   include `"-d" + depth + "-"`. Inside the recursive
   `nestedComponent`, set `depth: parent.depth + 1`. In the
   `submenuRequested` handler, replace the immediate
   `nestedLoader.item.open(...)` call with the
   `Loader.status === Loader.Ready` check + `Connections` listener
   pattern (see research.md Decision 4).
3. **`BarWidget.qml`** — extend `_sameTopLevel` to compare
   `children.length` and first-level child labels (FR-005). Wire
   `focusedScreenName` to consult `lastSnapshot.focused_output` as
   a fallback (FR-006 consume-side). Move the `_failedState` clear
   into a tiny helper and ensure it fires on every successful apply
   (FR-008).
4. **Tests.** Add `popup_geometry.qml` (FR-002, FR-003 assertions)
   and `submenu_cascade.qml` (FR-004, FR-007 assertions). Both use
   the same harness pattern as the existing
   `plugin/tests/qmltest/submenu_popup.qml`.
5. **Validate locally.**
   ```bash
   qmllint plugin/
   # Run qmltest harness — pattern from existing test:
   qmltestrunner -input plugin/tests/qmltest/popup_geometry.qml
   qmltestrunner -input plugin/tests/qmltest/submenu_cascade.qml
   ```
6. **Manual smoke.**
   ```bash
   qs -c noctalia-shell ipc reload
   # Open shadPS4QtLauncher; click View; verify dropdown rendered
   # with full-width labels visible. Hover Game List Mode; verify
   # cascade opens with List/Grid/Flat (or whatever the actual
   # children are). Click Settings on the bar; verify View popup
   # closes AND Settings popup opens in the same click.
   ```
7. **Commit + push + PR.**
   ```bash
   git add -- plugin/
   git commit -s -m "fix(qml): popup geometry, cascade, dedup (#009)"
   git fetch origin main && git rebase origin/main
   git push -u origin HEAD
   gh pr create --title "fix(qml): popup geometry, cascade, dedup" \
     --body "$(cat <<'EOF'
## Summary
- Constrain popup PanelWindow to menuBox extent (FR-002)
- Bind menuBox.width via childrenRect.width (FR-003)
- Wait for Loader.status === Ready before nested open (FR-004)
- Children-aware top-level dedup (FR-005)
- Cross-screen guard fallback to active.json focused_output (FR-006 consume-side)
- Depth-suffixed WlrLayershell namespace (FR-007)
- _failedState self-clear on successful apply (FR-008)

Spec: specs/009-popup-hotfix/

## Test plan
- [ ] qmllint plugin/ clean
- [ ] qmltestrunner popup_geometry.qml + submenu_cascade.qml green
- [ ] Manual smoke against shadPS4QtLauncher (SC-001..003)
- [ ] One of {kate, dolphin, krita} smoke for cross-app (SC-006)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
   ```

## Lane B — implementation walk-through

Files to edit:
- `bridge/src/atspi.rs`
- `bridge/src/active.rs`
- `bridge/src/focus.rs`
- `bridge/Cargo.toml`

Files to add:
- `bridge/tests/fixtures/qt_nested_wrapper.json`
- `bridge/tests/atspi_flatten.rs` (new) OR extend an existing
  `atspi_*` integration test

Step-by-step:

1. **`bridge/src/atspi.rs`** — find the existing wrapper-flatten
   block at lines 801-811 of `fetch_menu_tree`. Move it INSIDE
   the per-child fetch loop, immediately after the recursive
   `fetch_menu_tree(child_proxy)` returns and BEFORE the child
   is pushed to `item.children`. Each child is now flattened
   in isolation; the existing top-level call no longer needs
   the post-loop check.
2. **`bridge/src/focus.rs`** — surface the focused output's
   `name` from the niri-IPC focus event payload to the snapshot
   writer. Add a `pub fn focused_output(&self) -> Option<&str>`
   accessor or extend the existing focus state struct.
3. **`bridge/src/active.rs`** — extend the `ActiveSnapshot`
   serde struct with `focused_output: Option<String>`. Use
   `#[serde(skip_serializing_if = "Option::is_none")]` so v1.0.0
   consumers see no schema change when the field is absent.
4. **`bridge/tests/fixtures/qt_nested_wrapper.json`** — author a
   small JSON fixture mirroring the shadPS4QtLauncher View menu
   tree shape (with the unnamed-MENU wrapper at depth 1, 2, and
   3). See `contracts/recursive-flatten.md` for the test
   contract.
5. **`bridge/tests/atspi_flatten.rs`** — new test module that
   loads the fixture, applies the flatten algorithm (factor it
   out of `fetch_menu_tree` into a `pub(crate) fn flatten_qt_wrapper`
   so it's testable without an AT-SPI bus), asserts the post-
   flatten tree contains zero wrapper intermediates and the
   expected leaf labels at the expected depth.
6. **`bridge/Cargo.toml`** — bump `version = "1.0.1"`.
7. **Validate locally.**
   ```bash
   cargo test -p noctalia-appmenu-bridge --all-targets
   cargo clippy -p noctalia-appmenu-bridge -- -D warnings
   nix flake check .
   ```
8. **Manual smoke.**
   ```bash
   cargo build --release -p noctalia-appmenu-bridge
   # Restart user service:
   systemctl --user restart noctalia-appmenu-bridge.service
   # Focus shadPS4QtLauncher; check active.json:
   jq '.menu | .. | .label? // empty' ~/.cache/noctalia-appmenu/active.json | grep -c '""' # should be 0
   ```
9. **Commit + push + PR.**
   ```bash
   git add -- bridge/
   git commit -s -m "fix(bridge): recursive Qt menu flatten + focused_output (#009)"
   git fetch origin main && git rebase origin/main
   git push -u origin HEAD
   gh pr create --title "fix(bridge): recursive Qt menu flatten + focused_output" \
     --body "..."  # mirror Lane Q PR template
   ```

## Release walk-through (after both lanes merge)

1. From the main worktree on `main`:
   ```bash
   cd ~/Documents/Code/yolo-labz/noctalia-appmenu
   git fetch origin main && git pull --ff-only origin main
   ```
2. Verify `Cargo.toml` reads `version = "1.0.1"` and the bridge
   build is reproducible:
   ```bash
   nix flake check .
   ```
3. Cut tag:
   ```bash
   git tag -s v1.0.1 -m "v1.0.1 — popup hotfix (spec 009)"
   git push origin v1.0.1
   ```
4. Wait for the release workflow on the self-hosted runner. Verify:
   ```bash
   gh release view v1.0.1
   gh attestation verify ./noctalia-appmenu-bridge-linux-x86_64 \
     --repo yolo-labz/noctalia-appmenu
   ```
5. Update `~/NixOS/flake.lock` to point at v1.0.1's commit:
   ```bash
   cd ~/NixOS
   nix flake update noctalia-appmenu
   nh os switch .
   ```
6. Smoke against Pedro's actual setup (SC-001..006).

## Common pitfalls

- **`mapToGlobal` returns NaN before component is fully laid out.**
  Wrap in a `Component.onCompleted` defer if needed; or compute on
  first `openAt` call rather than as a binding.
- **`Loader.status === Loader.Ready` already true when handler
  fires.** Belt-and-braces: synchronous check first, `Connections`
  listener as fallback.
- **`childrenRect` includes hidden children.** Confirm `MenuRow`'s
  `visible: false` items don't inflate the width. If they do, gate
  with `visible: ...` on the children themselves, not the parent.
- **Cargo bump must NOT be amended onto the bridge fix commit.** The
  release workflow keys off `Cargo.toml` version; if the bump is in
  a separate squash-merge it lands cleanly. Same commit is fine
  too — both work.
