# Beyond Speckit — How to Actually Ship Good Software

**Context.** Pedro shipped 7 noctalia-appmenu releases (v1.0.0..v1.0.6) in 7 hours on 15-16/05/2026; the first 5 (v1.0.0..v1.0.4) silently failed to load because of a QML recursive-Component runtime error that `qmllint` accepted. The codex adversarial review eventually identified the root cause. This document captures the research synthesis on how to prevent the next cascade.

Each section is actionable for the next session.

## 1. Practices that would have caught the recursive-Component bug

`qmllint` is documented to **not** detect "instantiated recursively" — that error fires only when the QML *engine* instantiates the tree at runtime. The single most leverage-positive addition is a **runtime smoke test that loads the actual plugin in a real `QmlEngine`**.

Concrete forms:

- **`qmltestrunner` + a host harness** that imports `BarWidget.qml` from a `TestCase` and asserts `width > 0 && implicitWidth > 0` after `Component.onCompleted`. Extend the existing `plugin/tests/qmltest/submenu_popup.qml` pattern to every top-level QML component. Mock `qs.Commons` singletons.
- **`qml` CLI smoke** in CI: `nix develop --command qml -I plugin/ smoketest.qml --quit-after 2000` and fail on any stderr containing `instantiated recursively`, `TypeError`, `is not a type`. Runs in ~3s. **Highest leverage.**
- **Headless Wayland integration** with `cage` or `weston --backend=headless-backend.so` running a niri build, the bridge, and `qs -c noctalia-shell`. Hyprland already does this via `hyprtester` + `hyprtestplugin.so`.
- **Canary on dev host before tag.** Reorder the flow: build → `home-manager switch` to dev host → 5-minute soak with `journalctl -f` → only then `git tag`. Catches the entire cascade — runtime errors surface in seconds.

Property-based testing and visual diff are overkill for a single-shell menubar plugin. Skip both.

## 2. How mature Wayland/QML projects ship

- **Hyprland**: dual systems — `hyprland_gtests` (gtest unit) + `hyprtester` (integration against a live compositor) that loads `hyprtestplugin.so` and asserts internal dispatchers fire. Pinned via `commit_pins`.
- **end-4/dots-hyprland & caelestia-dots/shell**: zero formal CI tests — rely on dogfood + GitHub Issues. This is the *low* end of the bar and the pattern that bit us. **Don't model after them.**
- **Quickshell itself** uses hot-reload (`qs ipc reload`) as the dev loop. No upstream plugin-CI conformance suite.
- **AGS** has the same pattern (manual dogfood).

**Lesson:** ricing projects routinely ship broken; noctalia-appmenu has voluntarily climbed to a higher bar (SLSA, attestations, SonarQube). Finish the climb with a **runtime-load gate**.

## 3. Single-user dev-as-user best practices (2026)

Pedro IS the canary. Formalize:

1. **Two-stage tag flow**: cut `vX.Y.Z-rc.N` first, `nh os switch` to desktop, soak 30 min, then re-tag final. Already done for v1.0.0-rc.1/rc.2 → apply to *every* release, not just majors.
2. **Pre-push hook** runs `nix flake check` + qmltestrunner harness + a 2-second `qml` smoke load. ~10s cost; catches 90%.
3. **Reversibility**: nix generations + `nh os rollback` exist. Add a fast "previous-tag-flake" alias:
   ```nix
   home.shellAliases.napprev = "nix run github:yolo-labz/noctalia-appmenu/v${PREV}";
   ```
   so A/B compare in <5s.
4. **Observability**: structured `tracing` in the bridge — add per-event durations (`#[instrument(skip_all)]`) and a `dbus_method_duration_ms` histogram. The 500ms AT-SPI walk would have shown as `WARN walk took 547ms threshold=100ms` in the journal *before* Pedro noticed the freeze.

## 4. Defending against undocumented framework behaviour

`deleteOnInvisible: true` is the canonical case. Defences (in cost order):

- **Capture upstream behaviour in ADRs the moment you discover it.** Make this a hook: any bug fix that took >1 PR to land MUST produce an ADR documenting the upstream behaviour, with a link to the source code that proves it.
- **Contract test against a recorded fixture.** The bridge `niri.rs` already does fixture replay. Do the same for the Quickshell QML lifecycle: capture the property-write sequence and assert it. If Quickshell changes upstream, the fixture diverges loudly.
- **Pin Quickshell to a specific commit in `flake.lock`** (already done) and add a manual upgrade ritual: `nix flake update quickshell && run full integration suite` — never auto-bump.
- **Read the source, not the docs.** Quickshell docs are sparse; `git.outfoxxed.me/quickshell/quickshell` source is authoritative. Add a "source-read" check to the ADR template: cite a permalink, not a docs URL.

## 5. Concrete CI additions for `.github/workflows/`

Add a `plugin-runtime-smoke.yml` job (mirrors `plugin-lint`):

1. `nix develop --command qmltestrunner -input plugin/tests/qmltest` — extend existing harness to every top-level component.
2. ```bash
   nix develop --command timeout 5 qml -I plugin smoketest.qml 2>&1 | tee qml.log
   ! grep -E "instantiated recursively|TypeError|ReferenceError|Component is not ready" qml.log
   ```
3. Headless Wayland integration job: spin `cage -s -- niri --config tests/niri.kdl`, start the bridge, start `qs -c smoketest`, then `gdbus call --session --dest org.noctalia.AppMenu --object-path /org/noctalia/AppMenu/Active --method org.freedesktop.DBus.Introspectable.Introspect` — must return non-empty XML.
4. **`qmllint --strict`** in `ci.yml`.
5. **`nix flake check`** against `nixosConfigurations.test-vm` that boots a VM, enables the HM module, and asserts the bridge unit reaches `active (running)` — proves ADR-0027-class regressions never reach a tag.

## 6. The "7 broken releases in 7 hours" anti-pattern

Not formally named, but established literature:

- **"Fix-on-fix" / "cascading deployment failure"** ([Google SRE — Cascading Failures](https://sre.google/sre-book/addressing-cascading-failures/)). The SRE Book's *Managing Incidents* chapter walks through an incident "that spirals out of control due to ad hoc incident management" — verbatim our pattern. Their prescription: **freeze deploys, declare an incident, run a single coordinator, batch fixes**, not ship-as-you-find.
- **Change Failure Rate (CFR)** — *Accelerate* (Forsgren/Humble/Kim) defines this as % of deploys requiring rollback/hotfix. Elite teams: 0–15%. This session: 6/7 ≈ 86%. The DORA finding that matters: **speed and stability are positively correlated** — but only when each deploy is small AND tested. Our 7 deploys were small but untested at the runtime layer.
- **"Don't fix forward when you can roll back"** — [GoCD's hotfix vs rollback guide](https://www.gocd.org/2017/06/20/hotfixes-rollback-rollforward.html) + Continuous Delivery 2nd ed argue **rollback is the default**; fix-forward only when rollback is impossible. We could `nh os rollback` in <30s — make that the reflex.
- **Charity Majors' rule**: "If you've shipped >2 hotfixes in an hour, stop. Get observability first, then ship." (Honeycomb blog, paraphrased.)

**The repair:** add a **deploy-freeze rule** to CLAUDE.md — after 2 consecutive failed releases on the same feature, freeze tags for 24h, add the missing test surface, then resume.

---

## Sources

- [Google SRE — Cascading Failures](https://sre.google/sre-book/addressing-cascading-failures/)
- [Google SRE — Managing Incidents](https://sre.google/sre-book/managing-incidents/)
- [Hyprland Testing Framework (DeepWiki)](https://deepwiki.com/hyprwm/Hyprland/10.5-testing-framework)
- [Hyprland Tests Wiki](https://wiki.hypr.land/Contributing-and-Debugging/Tests/)
- [Qt qmllint docs](https://doc.qt.io/qt-6/qtqml-tooling-qmllint.html)
- [Qt Quick Test (qmltestrunner)](https://doc.qt.io/qt-6/qtquicktest-index.html)
- [Quickshell ProxyWindow Lifecycle](https://deepwiki.com/quickshell-mirror/quickshell/4.2-window-types-and-interfaces)
- [Quickshell PopupWindow docs](https://quickshell.org/docs/master/types/Quickshell/PopupWindow/)
- [Quickshell FAQ](https://quickshell.org/docs/v0.2.1/guide/faq/)
- [DORA / Accelerate metrics overview](https://waydev.co/accelerate-metrics/)
- [Aviator — Dogfood, Canary, Rollout](https://docs.aviator.co/releases-beta/concepts/dogfood-canary-and-rollout)
- [GoCD — Hotfixes, Rollback, Rollforward](https://www.gocd.org/2017/06/20/hotfixes-rollback-rollforward.html)
- [ariya.io — Pre-commit smoke testing](https://ariya.io/2012/03/git-pre-commit-hook-and-smoke-testing)
- [Continuous Delivery Patterns and Anti-Patterns (DZone)](https://dzone.com/refcardz/continuous-delivery-patterns)
- [WaylandTest framework notes](https://hackmd.io/@elkurin/B16xPAoeR)
- [end-4/dots-hyprland](https://github.com/end-4/dots-hyprland) / [caelestia-dots/shell](https://github.com/caelestia-dots/shell) (informational only — no CI smoke; do not model)
