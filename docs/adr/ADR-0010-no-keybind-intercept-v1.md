# ADR-0010 — No global Alt-F mnemonic intercept in v1

Status: Accepted
Date: 2026-05-04

## Context

macOS lets you press `Cmd+Shift+/` to open the application's Help menu, and Alt-letter mnemonics open menus by underlined letter. Implementing global Alt-F / Alt-E intercepts in noctalia would require:

1. A compositor-level keybind that fires before the focused app sees the keypress.
2. Mapping that keybind to a popup at runtime against the *currently rendered* menu's accelerator map.

niri's keybind config is static (KDL file) and does not support conditional binds. Wayland does not provide a global-grab API for non-compositor processes.

## Decision

No global mnemonic intercept in v1. Mouse and tab navigation only.

## Consequences

- **Positive:** Smaller v1 scope.
- **Negative:** Power-keyboard users miss the macOS-y Alt-F flow.
- **Mitigation:** A `xdotool`-style fallback recipe documented in README for users who want to wire their own niri keybind to "click the AppMenu widget's first child".

## Alternatives considered

- **Compositor-level keybind that pokes the bridge over D-Bus:** Works but every keybind is a per-user niri config edit; not portable. Deferred to v2.
- **Hijack `xkbcommon` on the noctalia surface:** Out of scope; would intercept user keypresses bound for the focused app.

## References

- [niri keybinds](https://github.com/YaLTeR/niri/wiki/Configuration:-Key-Bindings)
