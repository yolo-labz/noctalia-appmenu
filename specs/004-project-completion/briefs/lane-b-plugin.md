# Lane B worker brief — `006-plugin-completion`

You are a focused claude-code worker assigned **Lane B** of the `noctalia-appmenu` v1.0.0 roadmap. Read the source-of-truth files in the order listed, then proceed.

## Mission (one paragraph)

Land the QML plugin work (`plugin/*.qml`) for v1.0.0 per the umbrella spec `004-project-completion`. Specifically: create `plugin/SubmenuPopup.qml` (the nested-popup component that the existing v0.3.0 plugin's `AppmenuPopupWindow.qml:240` no-op refers to as "TODO alpha.19+"), wire it up, add `toggle_state` (checkmark) rendering, add `icon_name` (Qt icon-theme lookup) rendering in the popup row delegate, and add a multi-screen popup-routing guard so popups never open on the wrong screen. Implement under your own sub-spec at `specs/006-plugin-completion/`. Open no PR — push the branch and report.

## Source of truth (read in this order, all paths absolute)

1. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/spec.md` — read §User scenarios 1–4, §Functional requirements §Plugin, §Constraints, §SCs
2. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/plan.md` — §Approach + §Affected files §Lane B
3. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/research.md` — §3 (plugin audit findings)
4. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/contracts/submenu-popup-component.md` — full component contract
5. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu-004-project-completion/specs/004-project-completion/contracts/active-json-schema.md` — schema fields you render
6. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/specs/003-plugin-fault-isolation/spec.md` — fault-isolation invariants you inherit
7. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/docs/adr/ADR-0008-popup-window-for-submenus.md`, `ADR-0018-bar-widget-api-contract.md`, `ADR-0019-always-visible-bar-widget.md`, `ADR-0020-fixed-width-slot.md`
8. `/home/notroot/Documents/Code/yolo-labz/noctalia-appmenu/plugin/{BarWidget,AppmenuPopupWindow}.qml` (current state at v0.3.0 final)

## Your worktree

```bash
cd ~/Documents/Code/yolo-labz/noctalia-appmenu
git fetch origin main
git worktree add ../noctalia-appmenu-75-plugin-completion -b 75-plugin-completion origin/main
cd ../noctalia-appmenu-75-plugin-completion
```

If Lane A has already merged, rebase: `git fetch origin main && git rebase origin/main`.

## Your branch

`75-plugin-completion` off `origin/main`.

> Update branch number if PR # advances. Use `gh pr list --state all --limit 1` to confirm.

## FRs assigned to you

- **FR-010** create `plugin/SubmenuPopup.qml` (sibling top-level layer-shell `PanelWindow`, not nested); wire `hasChildren` click in `AppmenuPopupWindow.qml:240`
- **FR-011** render `toggle_state` checkmark indicator in popup row delegate
- **FR-012** render `icon_name` via Qt icon-theme lookup in popup row delegate
- **FR-013** multi-screen popup-routing guard — popup refuses to open if its `screen` mismatches the focused window's output

## Your speckit chain

```
specs/006-plugin-completion/{spec.md, plan.md, tasks.md, checklists/requirements.md}
```

Sub-spec is terse — cite spec 004 §FR-010..FR-013; ≤25 tasks.

## Hard constraints

1. **Worktree-first.** Never edit outside `noctalia-appmenu-75-plugin-completion/`.
2. **Branch off `origin/main`.** Not `004-project-completion`.
3. **DCO sign-off + conventional commits.** `git commit -s -m "feat(plugin): ..."`
4. **No push to `main`. No PR creation.**
5. **`qmllint plugin/*.qml` clean before committing each task.**
6. **Theme tokens only.** Use `Color.m*` and `Style.*` everywhere; no raw hex / rgb / Tailwind-style arbitrary spacing.
7. **Inherit spec 003 isolation invariants.** Any new `try { ... } catch (e) { ... }` envelope follows spec 003 FR-008 pattern. Nested popups MUST be sibling top-level `PanelWindow`s, NEVER nested inside the parent popup (spec 003 FR-005..FR-007).
8. **`WlrKeyboardFocus.None`** on all popups. No keyboard navigation in v1.
9. **Fixture test.** Add a QML test fixture (`plugin/tests/qmltest/submenu_popup.qml` or similar) that exercises the new component with a hand-crafted JSON tree.

## Allowlist of Bash commands

- `qmllint *` — single file or all
- `nix *` — `nix develop`, `nix flake check`
- `git status` / `git diff` / `git log` / `git add` / `git commit` / `git push` (your branch only) / `git fetch` / `git rebase` / `git worktree` / `git rev-parse` / `git branch`
- `gh pr list` / `gh pr view` / `gh pr checks` (NEVER `gh pr create`, NEVER `gh pr merge`)
- `ls`, `mkdir`, `find`, `test`, `stat`, `file`

## Acceptance gates

- [ ] `qmllint plugin/BarWidget.qml plugin/AppmenuPopupWindow.qml plugin/SubmenuPopup.qml` clean
- [ ] `nix flake check` passes
- [ ] Fixture test under `plugin/tests/qmltest/` exists and runs
- [ ] Manual smoke against the fixture confirms: nested popup opens; `toggle_state` indicator visible; icon renders; multi-screen guard refuses cross-screen open
- [ ] All commits DCO-signed
- [ ] Branch pushed

## Reporting

```
LANE B — plugin-completion: READY FOR PR
Branch: 75-plugin-completion
Commits: <N>
Last commit SHA: <sha>
Sub-spec dir: specs/006-plugin-completion/
Acceptance: <PASS/FAIL with one-line rationale>
Open items for PR review: <list>
```

## Anti-patterns

- ❌ Keyboard navigation, mnemonic underlines, Alt-F intercept (deferred to v2 per ADR-0010).
- ❌ Nested `Popup` items inside the parent popup (use sibling `PanelWindow` per ADR-0008).
- ❌ Animation that extends surface bounds beyond the reserved slot (spec 003 FR-002).
- ❌ `xdg_popup.grab(wl_seat)` — use full-screen `MouseArea` for outside-click instead.
- ❌ Raw hex colours or arbitrary Tailwind-style values.
