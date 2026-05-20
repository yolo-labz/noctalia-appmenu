# Case study — drift trigger I, multi-Firefox routing loop

**Spec:** `specs/015-ship-ready-completion/spec.md`
**Trigger:** I (user-reported failure mode persists across ≥ 2 deploys)
**Closes:** spec 015 FR-008 SC-005

## The loop

| Tag | PR | Axis | Pedro symptom report |
|---|---|---|---|
| v1.0.20 | #112 | Bridge — niri pre-focus with 30 ms settle | "new tab still opens on wrong Firefox instance" |
| v1.0.21 | #113 | Plugin — self-heal RefreshActive retry on empty children | "the problematic integration with multi instance programs persists" |
| v1.0.22 | #115 | Bridge — settle bump 30 ms → 150 ms | (would have been a third report if shipped without the trigger) |

Three releases. Three different *axes*. **One unchanged symptom**:
clicking *New Tab* on Firefox window A opens the tab on window B.

Drift triggers A-H all failed to catch this:

- **A (cites prior version)** — every commit cited the issue number
  (#109) and a unique technical axis, not the previous version's
  failure mode.
- **B (>3 releases in 24h)** — releases were ≥ 24h apart.
- **C (2 consecutive failed smokes)** — there was no smoke. Pedro
  was the regression detector.
- **D (Pedro repeats verbatim)** — Pedro's wording varied: "wrong
  Firefox instance" → "multi instance programs" → identical *symptom*,
  different *noun phrase*.
- **E (architectural revert)** — each patch moved forward, not back.
- **F (qmllint green, runtime red)** — both qmllint and cargo were
  green every time. The bug surfaces only at runtime, only at the
  Firefox-internal-command-dispatcher layer.
- **G (binary unchanged)** — `bridge --version` correctly matched
  the tag every deploy.
- **H ("should fix" without "verified by")** — commit bodies were
  careful, citing concrete mechanisms.

Triggers A-H watch the **implementation axis**. They miss the case
where every patch is genuinely different but the *user-visible
symptom* never changes — because the agent has misdiagnosed where
the bug lives.

## What trigger I catches

Trigger I watches the **symptom axis**: PR titles + bodies +
issue threads. If the same symptom phrase reappears ≥ 2 times
across recently-merged PRs, the next attempt must be a redesign
spec, not a third patch.

Mechanical detection:

```sh
gh issue list --state all --search "wrong Firefox instance"
# returns: #109, plus Pedro's chat re-files counted as evidence

gh pr list --state merged --search "wrong Firefox instance in:body"
# returns: #112 (settle introduction), #113 (self-heal), #115 (settle bump)
```

≥ 2 hits on both queries → trigger fires.

## What the redesign spec would say

If trigger I had been active when v1.0.22 was drafted, the
required output would have been a `specs/NNN-firefox-routing-redesign/spec.md`
documenting:

1. **Failure analysis per prior patch:**
   - v1.0.20: 30 ms is one vblank on 60Hz; Firefox needs ≥ 1 event
     loop tick *after* compositor focus swap to sync its internal
     active-browser pointer.
   - v1.0.21: RefreshActive retry fixes empty-children, NOT routing.
     Orthogonal axis — addressed a different bug.
   - v1.0.22: 150 ms is empirical 1-frame headroom for 60Hz, BUT does
     not address the root cause if Firefox's command dispatcher uses
     a per-process queue that doesn't drain on wl_keyboard.enter.

2. **Architectural alternatives:**
   - **A. Accelerator-key dispatch via compositor.** Wayland-spec
     dispatches Ctrl-T to the focused window directly. Bypasses
     Firefox's command dispatcher entirely. Works for accelerator-
     bearing leaves (covers New Tab, New Window, Close Tab).
   - **B. AT-SPI per-window accessible path.** Walk Firefox's
     accessible tree per window; address the New Tab leaf at
     `accessible/<window-N>/menubar/file/new-tab` rather than the
     global menu. Requires Firefox-specific tree mapping.
   - **C. Activate via D-Bus org.gtk.Application.** Firefox doesn't
     expose this. Dead end.
   - **D. xdg_activation token.** Reverse the flow: bridge captures
     an activation token from niri, passes it to the DoAction site
     so Firefox knows which window to bias. Investigation needed
     on whether AT-SPI surfaces this hook.

3. **Decision:** accelerator (A) for leaves with bindings; AT-SPI
   per-window (B) as fallback. Together they cover the user-visible
   surface without further iteration on settle time.

This is exactly what spec 015 FR-003 already proposes — but the
loop above shows the value of catching the drift mechanically rather
than relying on the next session's good judgement.

## Synthetic dry-run

To prove trigger I works without burning a real PR, this case study
itself is the dry-run input.

```sh
# Inside this branch (116-governance-and-backups):
git log -1 --format=%s
# Subject contains "drift trigger I" — does NOT overlap any prior
# merged PR's symptom phrase. Hook stays silent.

# Now synthetically rebrand the branch to a third routing patch:
git commit --amend -m "fix(bridge): try third settle bump for Firefox routing"
scripts/verify-no-third-patch.sh
# Hook detects the "Firefox routing" shingle, finds ≥ 2 prior merged
# PRs with that phrase (#112, #115), refuses the push with the
# redesign-spec template path.
```

The dry-run is verifiable; the synthetic amend can be reverted with
`git commit --amend` to restore the original subject.

## Pedro-facing remediation

When trigger I fires on Pedro's next session:

1. The pre-push hook prints the matching PRs and the redesign-spec
   template path.
2. Open the redesign worktree per the hook's instructions.
3. The new spec.md inherits this case study's "Failure analysis per
   prior patch" pattern as a section template.
4. Implementation only resumes after the redesign spec lands AND
   one merged PR has cited it as the implementation reference.

This case study is the contract for that flow.
