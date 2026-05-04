# ADR-0013 — Runner-agnostic CI labels + multi-runner pool

Status: Accepted
Date: 2026-05-04
Supersedes: ADR-0012 (partially — runner specifics)

## Context

ADR-0012 picked a single self-hosted runner on VM103 (4 vCPU, 11 GB RAM, 9 GB free disk). With 17+ jobs queued on the initial scaffold + first fix PR, the single-threaded runner became the throughput bottleneck — total queue clearance estimated at 1-2 hours wall-clock for the first PR cycle.

Pedro's desktop host (`x86_64-linux`, Ryzen 7950X3D, 32 threads, abundant RAM and disk) sits idle most of the time. Running CI work there in parallel with VM103 collapses queue clearance from hours to minutes.

The original workflow `runs-on:` labels included `vm103` as a host-pin: `[self-hosted, Linux, X64, vm103, noctalia-appmenu]`. That coupled jobs to a specific runner.

## Decision

1. Drop the host-specific label from `runs-on:`. New shape: `[self-hosted, Linux, X64, noctalia-appmenu]`. Any runner registered with the `noctalia-appmenu` project label is a valid host.
2. Register the desktop host as a second self-hosted runner. Same project label; host-identifier label `desktop` for ops visibility.
3. Keep VM103 registered too — it acts as a fallback when the desktop is heavily loaded with developer work.
4. Document `noctalia-appmenu` in `.github/actionlint.yaml` so static analysis doesn't flag it.

## Consequences

- **Positive:** Concurrent job execution. Cargo + Nix builds finish in roughly half the wall-clock time when both runners are online. Resilient to either runner being offline.
- **Negative:** CI jobs now run on the developer's desktop, sharing CPU / disk / network with foreground work. Mitigation: the runner systemd unit can be paused (`systemctl --user stop github-runner-noctalia-appmenu`) when crunching locally.
- **Negative:** Workflow expectations must remain runner-shape-agnostic. No assumptions about installed toolchain beyond what `nix develop` provides; no host-specific paths.
- **Mitigation:** ADR-0012 already mandated `nix develop --command` for all toolchain access. That decision now pays off — it makes runners interchangeable.

## Alternatives considered

- **Move CI entirely to desktop, retire VM103:** Rejected. VM103 is the fallback when the desktop is heavily loaded; losing it makes CI brittle.
- **Distributed Nix builds (desktop as remote builder for VM103):** Considered. More complex setup (SSH trust, `/etc/nix/machines`, NixOS config edit on both ends). Defer to a future ADR if the desktop-as-runner path proves insufficient.
- **GitHub-hosted runners only:** Rejected per ADR-0012 (integration tests need real niri).

## Rollout

1. Land this PR — workflows transition to `[self-hosted, Linux, X64, noctalia-appmenu]`.
2. Register the desktop runner with labels `self-hosted, Linux, X64, noctalia-appmenu, desktop`. systemd-user unit, hardened.
3. Verify by triggering a workflow with `workflow_dispatch`; confirm it lands on the desktop runner.
4. Re-run failed jobs from the bootstrap PR cycle on the new label set.

## References

- ADR-0012 — Self-hosted runner only (this ADR refines that decision)
- `meta/yolo-labz-release-engineering-research.md` — yolo-labz runner conventions
- `~/.claude/projects/-home-notroot-Documents-Code-CITi-DeliCasa/memory/vm103_github_runner.md` — VM103 topology
