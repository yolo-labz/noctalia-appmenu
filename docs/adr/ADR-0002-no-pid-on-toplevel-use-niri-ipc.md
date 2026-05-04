# ADR-0002 — No `pid` on Quickshell `Toplevel` — use niri-IPC bridge

Status: Accepted
Date: 2026-05-04

## Context

The bridge needs to map "currently focused Wayland surface" → "process PID" so it can find which app's menu the registrar holds. Quickshell's `Toplevel` exposes `appId`, `title`, `activated` — but no `pid` (verified against `quickshell/src/wayland/toplevel_management/handle.hpp:9-78`). Underlying protocol `zwlr_foreign_toplevel_handle_v1` does not carry pid; `ext-foreign-toplevel-list-v1` adds an opaque identifier but still no pid.

niri's IPC, on the other hand, exposes pid for every window via `niri msg --json windows` and emits `WindowFocusChanged{id}` in `event-stream`. So niri *does* know the pid, just not through Wayland.

## Decision

Use niri-IPC as the canonical focus-pid bridge. Do not heuristically match by `appId+title` — multiple windows of the same app collide (two qutebrowser windows, two Anki windows side-by-side).

## Consequences

- **Positive:** Reliable, exact pid matching. Survives two windows of the same app focused-and-blurred in quick succession.
- **Negative:** Couples us to niri's IPC schema. A schema change breaks us.
- **Mitigation:** Pin `niri-ipc` crate version in `Cargo.lock`. CI runs against the version matrix listed in `flake.nix`. Bridge logs schema mismatch and exits non-zero (caller systemd unit restarts).

## Alternatives considered

- **`appId+title` heuristic:** Rejected — collisions with two windows of the same app are inevitable; behaviour is non-deterministic.
- **Patch Quickshell to add `pid`:** Long road. `pid` would have to come from somewhere — either niri's IPC (in which case Quickshell vendors niri specifics) or a Wayland-protocol extension that does not exist yet. Rejected for v1; revisited if `xdg-dbus-annotation-v1` standardises.
- **Match on `wl_client_get_credentials`:** Server-side, not exposed to clients. Rejected.

## References

- `quickshell/src/wayland/toplevel_management/handle.hpp:9-78` — confirms no pid
- [niri IPC docs](https://yalter.github.io/niri/IPC.html)
- [niri-ipc crate](https://docs.rs/niri-ipc/)
