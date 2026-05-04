# ADR-0006 — Graceful degradation when no menu / no registrar

Status: Accepted
Date: 2026-05-04

## Context

Many apps will never publish to the registrar (Firefox, every Electron app, anything using GTK without `appmenu-gtk-module`). The bridge or registrar daemon may also be down at any moment. The widget must not crash the bar, log noisily, or render visual garbage in any of these cases.

## Decision

The widget renders one of three states, in priority order:

1. **Menu present** — render the registered menu tree.
2. **No menu, but app has a `.desktop` entry** — render a single dropdown with `App-name → About / Quit` derived from the desktop entry. The Quit action sends `SIGTERM` to the focused PID via niri-IPC.
3. **No menu, no `.desktop`** — render nothing (widget invisible).

The bridge similarly degrades:

- niri-IPC unreachable → exit non-zero (systemd restarts).
- Registrar offline → publish `(serviceName="", objectPath="")` for the active proxy; widget falls through to state 2 or 3.

## Consequences

- **Positive:** No bar crashes. No "menu disappeared mid-session" UX. Predictable.
- **Negative:** State 2 / 3 may surprise users who expected a full menu. We document this clearly.
- **Mitigation:** Tooltip on the pseudo-menu reads "App did not publish a menu — try setting `QT_QPA_PLATFORMTHEME=appmenu-qt5` or installing `appmenu-gtk-module-wayland`."

## Alternatives considered

- **Hide entirely on no-menu:** UX regression — the bar visibly empties when focus moves to Firefox, looks broken. Rejected.
- **Fail loud with a red error indicator:** Visually noisy, user can't act on it. Rejected.

## References

- [Quickshell.DesktopEntries](https://quickshell.org/docs/v0.3.0/types/Quickshell.Services.DesktopEntries/)
