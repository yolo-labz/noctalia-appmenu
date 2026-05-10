# ADR-0016 — niri event-stream JSON schema

- **Status:** Accepted (2026-05-04)
- **PR:** #23
- **Released in:** v0.1.4

## Context

The bridge subscribes to `niri msg --json event-stream`. v0.1.0..v0.1.3
modelled `NiriEvent` with serde's *internally-tagged* form
(`#[serde(tag = "type", rename_all = "PascalCase")]`), expecting events
shaped like `{"type": "WindowFocusChanged", "id": 7}`.

That assumption was wrong. niri 26.04 emits the *externally-tagged*
form: `{"WindowFocusChanged": {"id": 7}}`.

The error path was silent in normal operation: every event line failed
`serde_json::from_str::<NiriEvent>` with `missing field 'type'`, the
warn was logged, and the loop continued. Result: zero focus events
reached the proxy, `~/.cache/noctalia-appmenu/active.json` stayed at
its empty initial state, and the topbar widget rendered nothing on
every host running v0.1.0..v0.1.3 — a 100% silent regression that
shipped in three releases.

## Decision

Use serde's default *externally-tagged* enum form for `NiriEvent`.
Implement `Deserialize` manually so unknown variants fall through to a
`NiriEvent::Other` catch-all instead of erroring.

Manual impl is required because `#[serde(other)]` only works for
internally- and adjacently-tagged enums; externally-tagged enums have
no built-in catch-all. The manual impl deserialises into
`serde_json::Value`, then tries the typed variant set, mapping any
parse failure to `Other`. This keeps `from_str::<NiriEvent>(line)`
ergonomic at the call site and never crashes the event loop on schema
drift.

## Consequences

- `NiriEvent` is no longer `Deserialize`-derived; the manual impl is
  the schema surface and must move in lockstep with niri's wire format.
- Wire-format regression tests live in `bridge/src/niri.rs` (`#[cfg(test)]`)
  and exercise real journal samples captured from niri 26.04. Adding a
  new variant means: extend the `Typed` inner enum + add a sample test.
- Future niri versions that introduce new event variants will deserialize
  into `Other` (warn-and-skip) rather than crashing the bridge. The
  `Other` arm is `MapOp::NoOp` — the bridge ignores it, the systemd
  unit stays up.
- The negative-regression test
  (`internally_tagged_form_falls_through_to_other`) confirms the
  pre-v0.1.4 schema, if ever re-emitted upstream, will not crash the
  parser. It is not a wire-format expectation — niri does not emit
  that form.

## Alternatives considered

- `#[serde(untagged)]` with an explicit `Other(serde_json::Value)`
  catch-all: would work but pollutes the typed variants with an extra
  layer (`NiriEvent::Known(KnownEvent)`), making every match site uglier.
- Parsing into `serde_json::Map` and dispatching on the single key
  manually: more code than the `Value::deserialize` + `from_value` path
  chosen, no upside.
- Pinning niri's wire format via integration tests against a real niri:
  too heavy for unit-test coverage. Captured journal samples in
  `#[cfg(test)]` are sufficient and run in CI without a display server.

## References

- niri 26.04 IPC schema: `niri-ipc` crate sources, `event-stream` enum
- Pre-fix journal evidence:
  `journalctl --user -u noctalia-appmenu-bridge.service` on desktop
  showing repeated `could not parse niri event line: missing field 'type'`
- v0.1.4 release notes
