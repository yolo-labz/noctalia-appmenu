# niri-IPC contract

The bridge depends on niri's JSON IPC. Schema is pre-1.0 and may evolve; we pin the `niri-ipc` crate version in `Cargo.lock` and parse defensively.

## Interfaces we use

### `niri msg --json windows`

Returns `[NiriWindow]`. Per-window record fields we consume:

| Field | Type | Purpose |
|---|---|---|
| `id` | `u64` | Stable (per-niri-session) window identifier. |
| `app_id` | `string?` | Wayland app-id; informational only — never used to match windows. |
| `title` | `string?` | Title at snapshot time; informational only. |
| `pid` | `u32?` | PID of the wl_client owning the surface. **Load-bearing**. |

`workspace_id` and `is_focused` are kept on the deserialiser type for forward compatibility but `#[allow(dead_code)]`-suppressed today.

### `niri msg --json event-stream`

Streams JSON events, one per line. Event variants we consume:

| Variant | Carries | Bridge action |
|---|---|---|
| `WindowFocusChanged` | `{ id: u64? }` | Emit `FocusEvent` for matching `pid`; resync snapshot if `id` unknown. |
| `WindowOpenedOrChanged` | `{ window: NiriWindow }` | Upsert into the cache. |
| `WindowClosed` | `{ id: u64 }` | Remove from cache. |
| (other) | — | Ignored via `#[serde(other)]` catch-all. |

## Schema-drift defence

`niri.rs` separates schema-handling from side-effects. The pure transducer:

```rust
pub fn handle_event(event: NiriEvent, cache: &HashMap<u64, NiriWindow>) -> MapOp { ... }
```

returns one of seven `MapOp` variants — `FocusEmit`, `FocusUnknown`, `FocusNoPid`, `DefocusAll`, `Upsert`, `Remove`, `NoOp`. The `run()` task applies the op against its mutable cache.

This separation is what makes the niri-side schema unit-testable without spawning niri. See `bridge/src/niri.rs::tests`.

When niri ships a new event variant, our `#[serde(other)]` catches it and `handle_event` emits `MapOp::NoOp`. We can then add an explicit branch — without breaking deserialisation in flight.

## What happens when niri is unreachable

The bridge **hard-fails at startup** if `niri msg --json windows` errors out — see [ADR-0006](../adr/ADR-0006-graceful-degradation.md) for why we picked hard-fail over silent-empty-cache. systemd then restarts the unit; user sees the cause in `journalctl --user -u noctalia-appmenu-bridge.service`.

If niri's `event-stream` pipe closes mid-session (typical when niri itself exits), the bridge logs `"niri event-stream ended"` at warn level and exits non-zero. systemd restarts.

## Pin policy

`niri-ipc = "<version>"` in `bridge/Cargo.toml`. Lock file committed. Breaking schema changes get a fresh `bridge/src/niri.rs` PR with a regression test against the prior fixture.
