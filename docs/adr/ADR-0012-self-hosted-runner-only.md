# ADR-0012 — Self-hosted runner only; no public-CI matrix

Status: Accepted
Date: 2026-05-04

## Context

Integration tests need a working niri (Wayland compositor), Qt + GTK toolkits, and a D-Bus session. None of GitHub's hosted Linux runners ship niri. Setting it up with `xvfb-run` over X11 defeats the point — we are explicitly testing Wayland code paths.

VM 103 on `home302server` is already provisioned for yolo-labz / DeliCasa CI work, has the right toolchain, and sits behind the user's Tailscale + LAN.

## Decision

All CI runs on `runs-on: [self-hosted, Linux, X64, vm103, noctalia-appmenu]`. No matrix expansion to `ubuntu-latest`.

## Consequences

- **Positive:** Real niri test environment. Fast feedback loop (LAN, no public-CI quotas).
- **Negative:** Single point of failure. If VM 103 is down, CI is down.
- **Mitigation:** Document VM 103 recovery in `~/Documents/Notes/3. Resources/`. Bridge unit tests (`cargo test`, no display) run *also* on `ubuntu-latest` as a smoke check via a separate matrix-ed workflow file.

## Alternatives considered

- **GitHub-hosted runner with `xvfb-run` + headless Wayland:** Documented to be unreliable; `wayland-cage` works for tiny demos but breaks under multi-window load. Rejected.
- **Ephemeral cloud VM:** Cost. Rejected.

## References

- `~/.claude/projects/-home-notroot-Documents-Code-CITi-DeliCasa/memory/vm103_github_runner.md`
