//! noctalia-appmenu-bridge — sidecar daemon that joins niri's IPC focus
//! stream with the freedesktop AppMenu registrar and re-publishes the
//! active app's menu under a stable D-Bus address that the noctalia
//! QML widget can attach to.
//!
//! See `docs/adr/` for architectural decisions:
//!   - ADR-0001 vala-panel-appmenu as registrar
//!   - ADR-0002 niri-IPC bridge for PID resolution
//!   - ADR-0003 Rust sidecar (over pure-QML)
//!   - ADR-0004 PID-keyed registrar mapping
//!   - ADR-0007 Fixed proxy at constant address
//!   - ADR-0009 Debounce policy
//!
//! Library surface is exposed for integration tests; the production
//! entry point is `src/main.rs`.

#![warn(rustdoc::broken_intra_doc_links)]
#![forbid(unsafe_code)]

pub mod active;
pub mod config;
pub mod niri;
pub mod proxy;
pub mod registrar;
