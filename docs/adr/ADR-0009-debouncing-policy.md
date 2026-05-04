# ADR-0009 — Focus debounce 75 ms, registrar churn 250 ms

Status: Accepted
Date: 2026-05-04

## Context

niri's `WindowFocusChanged` events fire on every focus transition, including transient focus during keyboard navigation between two open windows of the same app. KDE's `KMainWindow` rebuilds its menubar in stages — emitting two or three `LayoutUpdated` signals within 100-200 ms when an app first registers. Forwarding both at full speed makes the widget flicker and burns CPU on every Alt-Tab.

## Decision

- Focus changes from niri-IPC: trail-edge debounce **75 ms**. After a focus change, wait 75 ms; if no further focus event arrives, emit. If another arrives, restart the timer.
- Registrar `LayoutUpdated` / `ItemsPropertiesUpdated`: trail-edge debounce **250 ms**. App is presumed busy rebuilding; emit one consolidated update after quiescence.

## Consequences

- **Positive:** Smooth UX during Alt-Tab and during KDE app cold-start. Lower CPU.
- **Negative:** Adds 75 ms perceptible latency on focus change. Acceptable.
- **Mitigation:** Tunables exposed via `~/.config/noctalia-appmenu-bridge/config.toml` so power users can drop the debounce to 0 if they want.

## Alternatives considered

- **No debounce:** Visible flicker on KDE app cold-start. Rejected.
- **Leading-edge debounce:** Same first-event latency, but the *second* event in a burst is dropped; would miss the final menu state. Rejected.

## References

- [`tokio::time::timeout`](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html) for the implementation primitive.
