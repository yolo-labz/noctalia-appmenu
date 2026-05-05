# ADR-0021 — Quickshell.Io.FileView exposes content via `text()` (call)

- **Status:** Accepted (2026-05-05)
- **PR:** #28
- **Released in:** v0.1.9

## Context

v0.1.6..v0.1.8 read the bridge's `active.json` via:

```qml
FileView {
    id: activeFile
    path: "…/active.json"
    onLoaded: {
        if (text.length === 0) { /* short-circuit */ }
        const j = JSON.parse(text);
        root.appId = j.app_id || "";
        // …
    }
}
```

This silently dropped every read. The widget loaded the property
contract, the bar reserved its slot, focus events flowed through the
bridge to `~/.cache/noctalia-appmenu/active.json` correctly — but
`appId` stayed empty and the placeholder `·` rendered no matter what.

Diagnostic Component.onCompleted + onAppIdChanged log on Pedro's
desktop showed:

```
APPMENU_DEBUG: completed widgetId=plugin:noctalia-appmenu section=left idx=1
               maxLabelWidth=200 implicitWidth=224 barHeight=37 screenName=DP-2
APPMENU_DEBUG: FileView onLoaded text.length=0
```

— `text.length=0` every reload, even immediately after `cat
active.json` showed 130+ chars of valid JSON.

## Root cause

`Quickshell.Io.FileView` does not expose file contents as a `text`
**property**. It exposes them as a `text()` **function call**.
Verified by reading the public QML stubs:

- `/nix/store/…-quickshell-…/lib/qt-6/qml/Quickshell/Io/FileView.qml`
  declares `path`, `preload`, `blockLoading`, `printErrors`,
  `watchChanges` as properties — but no `text` property.
- noctalia-shell's own usage in
  `Services/Theming/ColorSchemeService.qml::schemeReader` calls
  `JSON.parse(text())` (with the parentheses).

When QML reads `text` as a property in this context, it resolves the
identifier to a function reference, then string-coerces that to its
JS string representation (e.g. `function() { … }`). This long
non-empty string then `.length` evaluates non-zero in the
worth-doing case (and `JSON.parse` chokes on `function`), but
mysteriously empty in others depending on Quickshell version. On
Pedro's Quickshell (`2026-04-28_8742a7a`) the coercion produced an
empty string, hence `text.length === 0` was always true and the
short-circuit ran.

## Decision

Always call FileView's contents accessor as a function:

```qml
onLoaded: {
    const content = text();
    if (!content || content.length === 0) return;
    const j = JSON.parse(content);
    // …
}
```

The cached `content` local also prevents calling the function twice
per load (once for the length check, once for parse).

## Consequences

- The widget now actually populates `appId` from the bridge's JSON.
  v0.1.9 is the first release where the bar shows the focused app's
  id end-to-end. (v0.1.7 placeholder visibility + v0.1.8 fixed-width
  slot were both prerequisites that revealed THIS bug — once the
  layout race and visibility gate were resolved, the FileView read
  was the last hidden failure.)
- A defensive null-check `!content || content.length === 0` covers
  the (rare) case where Quickshell returns null for an unreadable
  file path. The `content.length === 0` check matches the bridge's
  initial-empty-snapshot write — which is now correctly handled
  rather than indistinguishable from a function-coercion miss.
- Inline ADR-0021 reference in the `onLoaded` body so future
  contributors don't accidentally drop the parens.

## Alternatives considered

- **Fall back to `Process { command: ["cat", path] }`:** works, but
  loses Quickshell's inotify-based `watchChanges` — would have to
  poll. Hot path; reject.
- **Subscribe directly via `Process { command: ["tail", "-f"] }`:**
  same poll problem + line buffering races.
- **Pin a different Quickshell version where `text` is a property:**
  Quickshell's API is unstable; pinning is fragile and would split
  from noctalia-shell's pin. Reject.

## References

- Quickshell `2026-04-28_8742a7a` — `lib/qt-6/qml/Quickshell/Io/FileView.qml`
  public QML stubs (no `text` property).
- noctalia-shell `Services/Theming/ColorSchemeService.qml::schemeReader`
  — canonical `JSON.parse(text())` usage.
- v0.1.7+v0.1.8 chain that exposed this bug after fixing the
  visibility / layout-cache races.
- v0.1.9 release notes.
