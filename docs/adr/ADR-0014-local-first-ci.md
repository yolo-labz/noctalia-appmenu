# ADR-0014 — Local-first CI ("prechew") via lefthook pre-push

Status: Accepted
Date: 2026-05-04
Related: ADR-0012 (self-hosted runner only), ADR-0013 (runner-agnostic)

## Context

After landing ADR-0013 the CI pool is two runners (VM103 + the developer's desktop) but each PR still serializes through whatever runner is free. The developer's desktop is a Ryzen 7950X3D with 32 threads, 64 GB RAM, and an idle CPU 90% of the time. Even with the second runner, a remote-first CI loop is a 5-minute wait between push and verdict — too slow for a tight inner loop.

The remote runner is the wrong place to discover "you forgot to run `cargo fmt`."

## Decision

Run **the full CI suite locally before every push**, in parallel, via `lefthook` `pre-push`. The remote runner becomes a verification safety net, not the gating critical path.

### Concretely

`lefthook.yml`'s `pre-push:` section runs every check the remote CI runs, in parallel, on the developer's hardware:

- `bridge-{clippy,test,doc,fmt}` — Rust gates
- `cargo-deny-{advisories,bans,licenses,sources}` — supply-chain
- `cargo-machete` — unused-deps
- `plugin-qmllint` — QML
- `nix-{flake-check,fmt-all,deadnix-all,statix-all}` — Nix
- `workflows-{actionlint,zizmor}` — GHA static analysis
- `semgrep-rust` — custom Rust rules
- `typos-all` — docs / identifier typos
- `gitleaks-all` — secret scan

Lefthook's `parallel: true` runs them concurrently. On the 7950X3D this completes in 30-90 s warm; 3-5 min cold (first time after a clean checkout).

The pre-push hook **refuses the push** on any failure. Mirrors what the remote runner would do, but in seconds instead of minutes.

### Justfile recipes

- `just shadow-ci` — runs the same `pre-push` pipeline manually without pushing.
- `just fix` — auto-fixes everything with an auto-fix path (`cargo fmt`, `alejandra`, `qmlformat`, `typos --write-changes`).
- `just ci-fast` — `shadow-ci` + a real `nix build .#noctalia-appmenu-bridge`.
- `just lefthook-install` — one-time hook installation post-clone.

## Consequences

- **Positive:** Most "CI failed" round-trips disappear. Developer sees the failure 5-30 s after push intent, not 5 min into a remote run.
- **Positive:** The remote runners spend their time on the **integration** tests that genuinely need a remote environment (real D-Bus session, niri-headless, full release-build matrix).
- **Positive:** Push-and-pray culture goes away — by construction, you know the suite passes when GitHub gets the commit.
- **Negative:** Slower first push of the day (cold nix-build artifacts; ~3-5 min). Acceptable — and avoidable: keep `nix develop` warm via direnv.
- **Negative:** The hook can be bypassed via `LEFTHOOK=0 git push` or `--no-verify`. CLAUDE.md hard-bans both; we trust the bypass is reserved for genuine emergencies (broken-runner recovery PRs).
- **Negative:** Hooks add ~3 s to every commit even for typo fixes (pre-commit subset). Mitigated by `parallel: true` and the staged-files glob filters.

## Alternatives considered

- **Pre-push with `--no-verify` allowed by default:** Rejected. The whole point is local validation; opt-out culture defeats it.
- **Custom git wrapper / Justfile target instead of lefthook:** Rejected. Lefthook is already a project dep, ships in the devShell, and is widely understood.
- **Distributed Nix builds with desktop as remote builder for VM103:** Considered. Higher operational cost (NixOS config edit on both ends, SSH key exchange) for marginal benefit on top of lefthook + ADR-0013. Filed under "future work" — revisit if the desktop-runner path proves insufficient.
- **Full CI as a shadow daemon on the desktop (push → desktop intercepts → mirrors checks → posts as PR check):** Overkill for a one-developer project. Lefthook achieves 90% of the value at 5% of the operational cost.

## Rollout

1. Land this PR — `lefthook.yml` gains the comprehensive `pre-push:` block.
2. Run `just lefthook-install` to wire the hooks into `.git/hooks/`.
3. Verify by pushing a noop commit; confirm all 18+ checks fire in parallel and complete green.
4. Document in `CONTRIBUTING.md` that `lefthook install` is mandatory after clone.

## References

- [lefthook docs](https://github.com/evilmartians/lefthook)
- ADR-0012 — Self-hosted runner only
- ADR-0013 — Runner-agnostic CI labels
- `~/.claude/CLAUDE.md` — hard ban on `--no-verify`
