//! `.desktop` fallback substrate (spec 016).
//!
//! AT-SPI (ADR-0024) is the authoritative menu source. It only works
//! for apps that expose a `MENU_BAR` accessible — Qt6/GTK with the a11y
//! bridge loaded. A large slice of a daily-driver app set exposes
//! nothing on the a11y bus at all: libcosmic/Iced (`cosmic-files`,
//! `cosmic-edit`), Electron without `--force-accessibility`
//! (Obsidian, Feishin, VS Code, Slack), Chromium/Chrome, Firefox, and
//! GTK4 popover-only apps. For those the bridge historically wrote
//! `{ "menu": null, "source": "empty" }` and the bar went blank.
//!
//! This module synthesises an **honest, identity-derived fallback
//! menu** for such apps, labelled `source = "desktop-fallback"` so no
//! consumer mistakes it for a real native menubar. It is built from:
//!
//! 1. the focused window's `app_id` (always present when a window is
//!    focused),
//! 2. the app's freedesktop `.desktop` entry (display name + `[Desktop
//!    Action]`s) when one can be resolved, and
//! 3. universal niri-IPC window controls (close / fullscreen / floating
//!    / move-workspace) — the same real compositor primitives the
//!    `atspi::synthetic_menu` minimal fallback already uses.
//!
//! ## Honesty contract (supersedes the v1.0.2 honest-or-hidden Empty,
//! ## ADR-0031)
//!
//! Every item maps to a **real** action:
//! - window controls → `niri msg action <verb>` (real compositor calls),
//! - `.desktop` actions → the app's own `Exec` from its trusted
//!   `.desktop` file, spawned as argv with field codes stripped,
//! - "New Window" → the entry's default `Exec`, same launcher.
//!
//! There are no faked keystroke / Cut-Copy-Paste items (the v1 UX trap
//! that PR #44 removed). A labelled, real-action fallback is honest;
//! the only thing it is *not* is the app's own in-window menu, and the
//! `source` field says exactly that.
//!
//! ## Security
//!
//! The launcher NEVER runs an `Exec` line through a shell. It tokenises
//! the `Exec` value per the freedesktop spec, strips field codes
//! (`%f %u %U …`), and spawns `argv[0]` with the remaining args
//! directly. The `active.json` menu carries only opaque
//! `<desktop-id>` / `<action-id>` tokens (never an `Exec` string), and
//! the click path re-resolves them against the trusted XDG application
//! dirs at click time — so a tampered cache file can at worst launch a
//! *different installed app*, never an arbitrary command. This is the
//! same trust model every freedesktop launcher (rofi, wofi, fuzzel)
//! operates under.

use crate::atspi::{
    niri_leaf, pretty_app_label, synthetic_submenu, synthetic_window_submenu, MenuItem,
    SYNTHETIC_SERVICE,
};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// A parsed `[Desktop Entry]` plus its `[Desktop Action …]` groups.
/// Only the fields the fallback menu needs are retained.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopEntry {
    /// Desktop id — the `.desktop` filename without extension
    /// (e.g. `com.system76.CosmicFiles`). Used verbatim on the click
    /// path to re-resolve the entry.
    pub id: String,
    /// `Name` — display name (e.g. `Files`, `Firefox`). May be empty.
    pub name: String,
    /// `Exec` — the default launch command, field codes intact.
    pub exec: String,
    /// `StartupWMClass` — used to match a Wayland `app_id` that differs
    /// from the desktop id.
    pub startup_wm_class: Option<String>,
    /// `Actions`/`[Desktop Action …]` entries, in `Actions=` order.
    pub actions: Vec<DesktopAction>,
}

/// One `[Desktop Action <id>]` group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopAction {
    /// Action id — the token after `Desktop Action ` in the group
    /// header (e.g. `new-window`).
    pub id: String,
    /// `Name` of the action (e.g. `New Window`).
    pub name: String,
    /// `Exec` of the action, field codes intact.
    pub exec: String,
}

// ── Parsing ──────────────────────────────────────────────────────────

/// Parse the textual content of a `.desktop` file into a [`DesktopEntry`].
///
/// `id` is the desktop id (filename without `.desktop`), supplied by the
/// caller since it is not in the file body. Returns `None` when the file
/// has no `[Desktop Entry]` group or is not `Type=Application`.
///
/// Only unlocalised keys are read (`Name`, not `Name[de]`) — locale
/// handling is deferred (see `docs/appmenu-state.md` follow-ups). First
/// occurrence of a key within a group wins, matching the freedesktop
/// recommendation for duplicate keys.
#[must_use]
pub fn parse_entry(content: &str, id: &str) -> Option<DesktopEntry> {
    // group header -> (key -> value), first value wins.
    let mut groups: Vec<(String, HashMap<String, String>)> = Vec::new();
    let mut cur: Option<usize> = None;

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            groups.push((header.to_string(), HashMap::new()));
            cur = Some(groups.len() - 1);
            continue;
        }
        let Some(gi) = cur else { continue };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        // Skip localised keys (`Name[de]`) — default locale only.
        if key.contains('[') {
            continue;
        }
        groups[gi]
            .1
            .entry(key.to_string())
            .or_insert_with(|| value.trim().to_string());
    }

    let entry_group = groups.iter().find(|(h, _)| h == "Desktop Entry")?.1.clone();

    // Only application launchers get a fallback. `Type` defaults to
    // absent on some hand-written files; treat missing Type as
    // Application (permissive) but reject explicit Link/Directory.
    if let Some(ty) = entry_group.get("Type") {
        if ty != "Application" {
            return None;
        }
    }

    let name = entry_group.get("Name").cloned().unwrap_or_default();
    let exec = entry_group.get("Exec").cloned().unwrap_or_default();
    let startup_wm_class = entry_group.get("StartupWMClass").cloned();

    // Ordered action ids from `Actions=a;b;c;`. Trailing `;` yields an
    // empty token we drop.
    let action_order: Vec<String> = entry_group
        .get("Actions")
        .map(|s| {
            s.split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let lookup_action = |aid: &str| -> Option<DesktopAction> {
        let header = format!("Desktop Action {aid}");
        let g = &groups.iter().find(|(h, _)| *h == header)?.1;
        Some(DesktopAction {
            id: aid.to_string(),
            name: g.get("Name").cloned().unwrap_or_default(),
            exec: g.get("Exec").cloned().unwrap_or_default(),
        })
    };

    let actions: Vec<DesktopAction> = if action_order.is_empty() {
        // No `Actions=` key: include any `[Desktop Action *]` groups in
        // file order (some hand-written files omit the index key).
        groups
            .iter()
            .filter_map(|(h, _)| h.strip_prefix("Desktop Action ").map(str::to_string))
            .filter_map(|aid| lookup_action(&aid))
            .collect()
    } else {
        action_order
            .iter()
            .filter_map(|a| lookup_action(a))
            .collect()
    };

    Some(DesktopEntry {
        id: id.to_string(),
        name,
        exec,
        startup_wm_class,
        actions,
    })
}

/// Whether a parsed entry should be offered as a fallback. `NoDisplay`
/// and `Hidden` entries are not user-facing launchers, so we treat them
/// as "no entry" and fall through to the minimal identity fallback.
fn is_displayable(content: &str) -> bool {
    // Re-scan the `[Desktop Entry]` group for the two boolean keys.
    // Cheap and keeps `DesktopEntry` free of presentation-only fields.
    let mut in_entry = false;
    let mut no_display = false;
    let mut hidden = false;
    for raw in content.lines() {
        let line = raw.trim();
        if let Some(h) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            in_entry = h == "Desktop Entry";
            continue;
        }
        if !in_entry {
            continue;
        }
        if line == "NoDisplay=true" {
            no_display = true;
        } else if line == "Hidden=true" {
            hidden = true;
        }
    }
    !(no_display || hidden)
}

// ── Exec field-code handling ─────────────────────────────────────────

/// Tokenise a `.desktop` `Exec` value into argv, stripping field codes.
///
/// Implements the freedesktop "Exec variables" rules needed for a
/// no-argument launch:
/// - double-quoted args may contain spaces; `\\` `\"` `\`` `\$` are
///   unescaped inside quotes,
/// - `%%` becomes a literal `%`,
/// - every field code (`%f %F %u %U %d %D %n %N %i %c %k %v %m`) is
///   dropped — we launch with no file/URI arguments.
///
/// Returns the argv; `argv[0]` is the program. Empty when `exec` has no
/// real tokens.
#[must_use]
pub fn exec_to_argv(exec: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut have_token = false;
    let mut chars = exec.chars().peekable();
    let mut in_quote = false;

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quote = !in_quote;
                have_token = true;
            }
            '\\' if in_quote => {
                // Inside quotes a backslash escapes the next reserved char.
                if let Some(&next) = chars.peek() {
                    if matches!(next, '"' | '`' | '$' | '\\') {
                        cur.push(next);
                        chars.next();
                        continue;
                    }
                }
                cur.push('\\');
            }
            '%' => {
                // `%%` is a literal percent; every other field code
                // (`%f %u %U …`) consumes its letter and expands to
                // nothing — we launch with no file/URI arguments.
                if let Some('%') = chars.next() {
                    cur.push('%');
                    have_token = true;
                }
            }
            ' ' | '\t' if !in_quote => {
                if have_token {
                    args.push(std::mem::take(&mut cur));
                    have_token = false;
                }
            }
            other => {
                cur.push(other);
                have_token = true;
            }
        }
    }
    if have_token {
        args.push(cur);
    }
    args.into_iter().filter(|a| !a.is_empty()).collect()
}

// ── Discovery + resolution ───────────────────────────────────────────

/// XDG application directories, in lookup precedence order:
/// `$XDG_DATA_HOME/applications` then each `$XDG_DATA_DIRS/applications`.
///
/// On NixOS the per-user + system profile `share` dirs are already part
/// of `XDG_DATA_DIRS` (`/run/current-system/sw/share`,
/// `~/.nix-profile/share`, `/etc/profiles/per-user/$USER/share`), which a
/// niri/noctalia session always exports — so honouring `XDG_DATA_DIRS` is
/// the NixOS-correct discovery path. The hardcoded
/// `"/usr/local/share:/usr/share"` is the freedesktop-spec default used
/// ONLY when `XDG_DATA_DIRS` is entirely unset (it would miss the Nix
/// profiles, but that env state does not occur in a real session).
#[must_use]
pub fn app_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
        });
    if let Some(h) = data_home {
        dirs.push(h.join("applications"));
    }

    let data_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    let data_dirs = if data_dirs.is_empty() {
        "/usr/local/share:/usr/share".to_string()
    } else {
        data_dirs
    };
    for d in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(d).join("applications"));
    }
    dirs
}

/// Normalise a Wayland `app_id` or desktop id for fuzzy comparison:
/// lowercase, strip a reverse-DNS prefix (`com.system76.cosmicfiles` →
/// `cosmicfiles`). Single-dot ids (`foo.bar`) are preserved.
fn normalize(s: &str) -> String {
    let lower = s.trim().to_lowercase();
    match lower.rfind('.') {
        Some(idx) if lower[..idx].contains('.') => lower[idx + 1..].to_string(),
        _ => lower,
    }
}

/// Read + parse a single candidate `.desktop` file, honouring
/// NoDisplay/Hidden. Returns `None` on any read/parse failure.
fn read_candidate(path: &Path, id: &str) -> Option<DesktopEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    if !is_displayable(&content) {
        return None;
    }
    parse_entry(&content, id)
}

/// Whether `id` is a safe desktop / app identifier to interpolate into a
/// `<dir>/<id>.desktop` path. **Security boundary** (ADR-0031): the
/// resolver builds that path by `Path::join`, and `Path::join` with an
/// absolute or `..`-laden component would escape the trusted XDG dirs
/// (an absolute component *replaces* the base entirely). A tampered
/// `active.json` carrying `xdg:/tmp/pwn` or `xdg:../../etc/x` could then
/// have an arbitrary `.desktop`'s `Exec` spawned. freedesktop desktop
/// ids are bare reverse-DNS tokens, so we whitelist `[A-Za-z0-9._-]`
/// (which excludes `/`, whitespace, and absolute paths) and reject any
/// `..` path component. Wayland `app_id`s fit the same shape; an exotic
/// one simply degrades to the minimal identity fallback.
#[must_use]
fn is_valid_desktop_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 255
        && id != ".."
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Resolve an `app_id` to a [`DesktopEntry`] by searching `dirs`.
///
/// Split out from [`resolve`] so tests can point it at a fixture
/// directory without mutating process env. Resolution ladder:
///
/// 1. **Direct id** — `<dir>/<app_id>.desktop` (covers reverse-DNS apps
///    like `com.system76.CosmicFiles`, and `firefox`, `org.kde.kate`).
/// 2. **Scan** the dirs once and match by, in priority:
///    `StartupWMClass` == app_id, then desktop-id == app_id, then
///    `Exec` basename, then normalised `Name`/id fuzzy-equality.
///
/// First displayable hit wins; directories earlier in `dirs` shadow
/// later ones (XDG precedence). Returns `None` for an id that fails
/// [`is_valid_desktop_id`] — the path-traversal guard.
#[must_use]
pub fn resolve_in(dirs: &[PathBuf], app_id: &str) -> Option<DesktopEntry> {
    let app_id = app_id.trim();
    if !is_valid_desktop_id(app_id) {
        return None;
    }

    // Pass 1: direct filename hit (cheap stat, no full scan). `app_id`
    // is validated above, so this join cannot escape `dir`.
    for dir in dirs {
        let candidate = dir.join(format!("{app_id}.desktop"));
        if candidate.is_file() {
            if let Some(e) = read_candidate(&candidate, app_id) {
                return Some(e);
            }
        }
    }

    // Pass 2: scan. Collect the best match by priority tier.
    let norm_id = normalize(app_id);
    let mut by_wmclass: Option<DesktopEntry> = None;
    let mut by_exec: Option<DesktopEntry> = None;
    let mut by_fuzzy: Option<DesktopEntry> = None;

    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        for ent in rd.flatten() {
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some(parsed) = read_candidate(&path, stem) else {
                continue;
            };

            if by_wmclass.is_none()
                && parsed
                    .startup_wm_class
                    .as_deref()
                    .is_some_and(|w| w.eq_ignore_ascii_case(app_id))
            {
                by_wmclass = Some(parsed);
                continue; // highest tier; keep scanning only to satisfy earlier-dir precedence is already handled by order
            }
            if by_exec.is_none() {
                let exec_base = exec_to_argv(&parsed.exec)
                    .first()
                    .and_then(|p| Path::new(p).file_name().and_then(|s| s.to_str()))
                    .map(normalize);
                if exec_base.as_deref() == Some(norm_id.as_str()) {
                    by_exec = Some(parsed.clone());
                }
            }
            if by_fuzzy.is_none()
                && (normalize(&parsed.id) == norm_id || normalize(&parsed.name) == norm_id)
            {
                by_fuzzy = Some(parsed);
            }
        }
        // A wmclass hit in an earlier dir wins outright.
        if by_wmclass.is_some() {
            break;
        }
    }

    by_wmclass.or(by_exec).or(by_fuzzy)
}

/// TTL for the per-`app_id` resolution memo. Re-resolving picks up newly
/// installed `.desktop` files within this window without re-scanning the
/// application dirs on every focus event.
const RESOLVE_TTL: Duration = Duration::from_secs(60);

struct Memo {
    entry: Option<DesktopEntry>,
    at: Instant,
}

static RESOLVE_CACHE: LazyLock<Mutex<HashMap<String, Memo>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Resolve an `app_id` to a [`DesktopEntry`] against the live XDG dirs,
/// memoised for [`RESOLVE_TTL`]. Negative results are cached too so a
/// no-`.desktop` app does not re-scan every focus event.
#[must_use]
pub fn resolve(app_id: &str) -> Option<DesktopEntry> {
    let key = app_id.trim().to_string();
    // Path-traversal guard (see `is_valid_desktop_id`) — reject before we
    // touch the filesystem or memoise a junk key.
    if !is_valid_desktop_id(&key) {
        return None;
    }
    if let Ok(cache) = RESOLVE_CACHE.lock() {
        if let Some(m) = cache.get(&key) {
            if m.at.elapsed() < RESOLVE_TTL {
                return m.entry.clone();
            }
        }
    }
    let entry = resolve_in(&app_dirs(), &key);
    if let Ok(mut cache) = RESOLVE_CACHE.lock() {
        cache.insert(
            key,
            Memo {
                entry: entry.clone(),
                at: Instant::now(),
            },
        );
    }
    entry
}

// ── Fallback menu construction ───────────────────────────────────────

/// A `.desktop`-action leaf. Click path `xdg-action:<id>:<action-id>`.
fn action_leaf(idx: i32, label: &str, desktop_id: &str, action_id: &str) -> MenuItem {
    MenuItem {
        id: idx,
        label: label.to_string(),
        item_type: "standard".to_string(),
        enabled: true,
        visible: true,
        service: SYNTHETIC_SERVICE.to_string(),
        path: format!("xdg-action:{desktop_id}:{action_id}"),
        ..Default::default()
    }
}

/// Default-launch leaf ("New Window"). Click path `xdg:<id>`.
fn launch_leaf(idx: i32, label: &str, desktop_id: &str) -> MenuItem {
    MenuItem {
        id: idx,
        label: label.to_string(),
        item_type: "standard".to_string(),
        enabled: true,
        visible: true,
        service: SYNTHETIC_SERVICE.to_string(),
        path: format!("xdg:{desktop_id}"),
        ..Default::default()
    }
}

/// A non-clickable separator row within a submenu.
fn separator(idx: i32) -> MenuItem {
    MenuItem {
        id: idx,
        item_type: "separator".to_string(),
        enabled: false,
        visible: true,
        ..Default::default()
    }
}

/// Build the enriched fallback for an app that resolved to a
/// `.desktop` entry: an `<App>` button (desktop actions + New Window +
/// Quit) and a `Window` button (niri controls).
#[must_use]
fn build_enriched(app_id: &str, entry: &DesktopEntry) -> MenuItem {
    // Prefer the entry's own display Name; fall back to a prettified
    // app_id when the entry has no Name.
    let pretty = if entry.name.trim().is_empty() {
        pretty_app_label(app_id)
    } else {
        entry.name.trim().to_string()
    };

    let mut kids: Vec<MenuItem> = Vec::new();
    let mut idx = 0;
    if entry.actions.is_empty() {
        // No declared actions: synthesise a single default-launch item
        // so the app menu is still useful (e.g. Obsidian, a terminal).
        if !entry.exec.trim().is_empty() {
            kids.push(launch_leaf(idx, "New Window", &entry.id));
            idx += 1;
        }
    } else {
        // The app declares its own actions (e.g. Chrome's "New Window"
        // / "New Incognito Window"). Surface those verbatim and skip the
        // synthetic default-launch item — it would duplicate the first
        // action's label and lie about being distinct.
        for action in &entry.actions {
            let label = if action.name.trim().is_empty() {
                &action.id
            } else {
                action.name.trim()
            };
            kids.push(action_leaf(idx, label, &entry.id, &action.id));
            idx += 1;
        }
    }
    // macOS-style: separate the app/launch items from Quit.
    if !kids.is_empty() {
        kids.push(separator(idx));
        idx += 1;
    }
    // Quit maps to niri close-window — never SIGKILL (honest + safe).
    kids.push(niri_leaf(idx, &format!("Quit {pretty}"), "close-window"));

    let app_submenu = synthetic_submenu(0, &pretty, kids);
    let window_submenu = synthetic_window_submenu();

    MenuItem {
        id: 0,
        label: pretty,
        item_type: "submenu".to_string(),
        enabled: true,
        visible: true,
        service: SYNTHETIC_SERVICE.to_string(),
        path: "niri:noop".to_string(),
        children: vec![app_submenu, window_submenu],
        ..Default::default()
    }
}

/// Build the desktop fallback menu for `app_id`, or `None` when no
/// identity is available (empty `app_id`).
///
/// Priority:
/// - `app_id` resolves to a displayable `.desktop` entry →
///   [`build_enriched`] (name + actions + launch + window controls),
/// - `app_id` present but unresolved → the minimal identity fallback
///   ([`crate::atspi::synthetic_menu`]: app name + window controls),
/// - `app_id` empty → `None` (caller emits `source = "empty"`).
///
/// The caller ([`crate::proxy`]) only reaches here AFTER AT-SPI has
/// returned no menu, so this never shadows a real native menubar.
#[must_use]
pub fn fallback_menu(app_id: &str) -> Option<MenuItem> {
    let trimmed = app_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    match resolve(trimmed) {
        Some(entry) => Some(build_enriched(trimmed, &entry)),
        None => Some(crate::atspi::synthetic_menu(trimmed)),
    }
}

// ── Safe launchers (click path) ──────────────────────────────────────

/// Spawn an argv detached from this short-lived process. NO shell.
/// Returns an error if `argv` is empty or the program cannot be spawned.
fn spawn_detached(argv: Vec<String>) -> Result<()> {
    let (prog, rest) = argv
        .split_first()
        .ok_or_else(|| anyhow!("empty Exec argv"))?;
    // Drop the child handle: the launched app re-parents to init/systemd
    // when this `atspi-click` subprocess exits. `kill_on_drop` defaults
    // to false, so dropping does not signal the child.
    tokio::process::Command::new(prog)
        .args(rest)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("spawning {prog}"))?;
    Ok(())
}

/// Launch the default action of the app with desktop id `id`
/// (`xdg:<id>` click path). Re-resolves the entry against the trusted
/// XDG dirs, parses its `Exec`, and spawns argv with field codes
/// stripped.
pub async fn launch_app(id: &str) -> Result<()> {
    let entry = resolve(id).ok_or_else(|| anyhow!("no .desktop entry for id {id:?}"))?;
    let argv = exec_to_argv(&entry.exec);
    if argv.is_empty() {
        anyhow::bail!("desktop entry {id:?} has no runnable Exec");
    }
    spawn_detached(argv)
}

/// Launch the `[Desktop Action <action_id>]` of the app with desktop id
/// `id` (`xdg-action:<id>:<action_id>` click path).
pub async fn launch_action(id: &str, action_id: &str) -> Result<()> {
    let entry = resolve(id).ok_or_else(|| anyhow!("no .desktop entry for id {id:?}"))?;
    let action = entry
        .actions
        .iter()
        .find(|a| a.id == action_id)
        .ok_or_else(|| anyhow!("desktop entry {id:?} has no action {action_id:?}"))?;
    let argv = exec_to_argv(&action.exec);
    if argv.is_empty() {
        anyhow::bail!("action {action_id:?} of {id:?} has no runnable Exec");
    }
    spawn_detached(argv)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COSMIC: &str = "\
[Desktop Entry]
Type=Application
Name=Files
Exec=cosmic-files %U
Icon=com.system76.CosmicFiles
StartupWMClass=com.system76.CosmicFiles
Actions=new-window;

[Desktop Action new-window]
Name=New Window
Exec=cosmic-files --new-window
";

    #[test]
    fn parse_extracts_entry_and_actions() {
        let e = parse_entry(COSMIC, "com.system76.CosmicFiles").unwrap();
        assert_eq!(e.id, "com.system76.CosmicFiles");
        assert_eq!(e.name, "Files");
        assert_eq!(e.exec, "cosmic-files %U");
        assert_eq!(
            e.startup_wm_class.as_deref(),
            Some("com.system76.CosmicFiles")
        );
        assert_eq!(e.actions.len(), 1);
        assert_eq!(e.actions[0].id, "new-window");
        assert_eq!(e.actions[0].name, "New Window");
        assert_eq!(e.actions[0].exec, "cosmic-files --new-window");
    }

    #[test]
    fn parse_orders_actions_by_actions_key() {
        let content = "\
[Desktop Entry]
Type=Application
Name=Term
Exec=term
Actions=second;first;

[Desktop Action first]
Name=First
Exec=term --first

[Desktop Action second]
Name=Second
Exec=term --second
";
        let e = parse_entry(content, "term").unwrap();
        // `Actions=second;first` order is honoured, not file order.
        assert_eq!(e.actions.len(), 2);
        assert_eq!(e.actions[0].id, "second");
        assert_eq!(e.actions[1].id, "first");
    }

    #[test]
    fn parse_collects_actions_without_actions_key() {
        let content = "\
[Desktop Entry]
Type=Application
Name=Term
Exec=term

[Desktop Action solo]
Name=Solo
Exec=term --solo
";
        let e = parse_entry(content, "term").unwrap();
        assert_eq!(e.actions.len(), 1);
        assert_eq!(e.actions[0].id, "solo");
    }

    #[test]
    fn parse_rejects_non_application_type() {
        let content = "[Desktop Entry]\nType=Link\nName=X\nURL=https://e.com\n";
        assert!(parse_entry(content, "x").is_none());
    }

    #[test]
    fn parse_ignores_comments_localised_keys_and_blank_lines() {
        let content = "\
# a comment
[Desktop Entry]

Type=Application
Name=Real
Name[de]=Unecht
Exec=real
";
        let e = parse_entry(content, "real").unwrap();
        assert_eq!(e.name, "Real"); // not the [de] value
    }

    #[test]
    fn parse_first_value_wins_for_duplicate_keys() {
        let content = "[Desktop Entry]\nType=Application\nName=First\nName=Second\nExec=x\n";
        let e = parse_entry(content, "x").unwrap();
        assert_eq!(e.name, "First");
    }

    #[test]
    fn parse_malformed_returns_none() {
        // No [Desktop Entry] group at all.
        assert!(parse_entry("just some text\nno groups here\n", "x").is_none());
        assert!(parse_entry("", "x").is_none());
    }

    #[test]
    fn displayable_filters_nodisplay_and_hidden() {
        assert!(is_displayable(COSMIC));
        assert!(!is_displayable(
            "[Desktop Entry]\nType=Application\nName=X\nNoDisplay=true\n"
        ));
        assert!(!is_displayable(
            "[Desktop Entry]\nType=Application\nName=X\nHidden=true\n"
        ));
        // NoDisplay in an action group must NOT hide the entry.
        assert!(is_displayable(
            "[Desktop Entry]\nType=Application\nName=X\n\n[Desktop Action a]\nNoDisplay=true\n"
        ));
    }

    #[test]
    fn exec_argv_strips_field_codes() {
        assert_eq!(exec_to_argv("cosmic-files %U"), vec!["cosmic-files"]);
        assert_eq!(exec_to_argv("foo %f %u %i %c %k"), vec!["foo"]);
        assert_eq!(
            exec_to_argv("app --flag %F --other"),
            vec!["app", "--flag", "--other"]
        );
    }

    #[test]
    fn exec_argv_handles_quotes_and_percent_escape() {
        assert_eq!(
            exec_to_argv(r#""/opt/My App/bin" --run"#),
            vec!["/opt/My App/bin", "--run"]
        );
        // %% -> literal %
        assert_eq!(exec_to_argv("foo 50%%"), vec!["foo", "50%"]);
        // escaped quote inside quotes
        assert_eq!(exec_to_argv(r#""a\"b""#), vec![r#"a"b"#]);
    }

    #[test]
    fn exec_argv_empty_is_empty() {
        assert!(exec_to_argv("").is_empty());
        assert!(exec_to_argv("   ").is_empty());
        assert!(exec_to_argv("%f %u").is_empty());
    }

    #[test]
    fn normalize_strips_reverse_dns() {
        assert_eq!(normalize("com.system76.CosmicFiles"), "cosmicfiles");
        assert_eq!(normalize("org.kde.kate"), "kate");
        assert_eq!(normalize("firefox"), "firefox");
        assert_eq!(normalize("anki.bin"), "anki.bin"); // single dot kept
    }

    // ── resolve_in against an on-disk fixture dir (env-free) ──────────

    fn write_desktop(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn resolve_in_direct_id_hit() {
        let tmp = tempfile::tempdir().unwrap();
        write_desktop(tmp.path(), "com.system76.CosmicFiles.desktop", COSMIC);
        let e = resolve_in(&[tmp.path().to_path_buf()], "com.system76.CosmicFiles").unwrap();
        assert_eq!(e.name, "Files");
    }

    #[test]
    fn resolve_in_startup_wm_class_match() {
        let tmp = tempfile::tempdir().unwrap();
        // File named differently from the app_id; matched via StartupWMClass.
        write_desktop(tmp.path(), "files.desktop", COSMIC);
        let e = resolve_in(&[tmp.path().to_path_buf()], "com.system76.CosmicFiles").unwrap();
        assert_eq!(e.id, "files");
        assert_eq!(e.name, "Files");
    }

    #[test]
    fn resolve_in_skips_hidden_entry() {
        let tmp = tempfile::tempdir().unwrap();
        write_desktop(
            tmp.path(),
            "ghost.desktop",
            "[Desktop Entry]\nType=Application\nName=Ghost\nExec=ghost\nNoDisplay=true\n",
        );
        assert!(resolve_in(&[tmp.path().to_path_buf()], "ghost").is_none());
    }

    #[test]
    fn resolve_in_unknown_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        write_desktop(tmp.path(), "com.system76.CosmicFiles.desktop", COSMIC);
        assert!(resolve_in(&[tmp.path().to_path_buf()], "org.nope.Missing").is_none());
    }

    #[test]
    fn valid_desktop_id_accepts_real_ids_rejects_traversal() {
        for ok in [
            "com.system76.CosmicFiles",
            "firefox-nightly",
            "org.kde.kate",
            "google-chrome",
            "obsidian",
        ] {
            assert!(is_valid_desktop_id(ok), "should accept {ok}");
        }
        for bad in [
            "",
            "..",
            "/tmp/pwn",
            "../../etc/shadow",
            "a/b",
            "a b",
            "a;rm -rf",
            "a$(x)",
            "name\ninjected",
        ] {
            assert!(!is_valid_desktop_id(bad), "should reject {bad:?}");
        }
        // Length bound.
        assert!(!is_valid_desktop_id(&"a".repeat(256)));
    }

    #[test]
    fn resolve_in_rejects_path_traversal_even_when_target_exists() {
        // BLOCKER guard (codex #160): a tampered id must not escape the
        // search dirs via an absolute path or `..`. Plant a real file in
        // a sibling dir and confirm a traversal id cannot reach it.
        let dirs_root = tempfile::tempdir().unwrap();
        let search = dirs_root.path().join("search");
        let evil = dirs_root.path().join("evil");
        std::fs::create_dir_all(&search).unwrap();
        std::fs::create_dir_all(&evil).unwrap();
        write_desktop(&evil, "pwn.desktop", COSMIC);

        // Absolute path id.
        let abs = evil.join("pwn");
        let search_dirs = std::slice::from_ref(&search);
        assert!(resolve_in(search_dirs, abs.to_str().unwrap()).is_none());
        // Relative traversal id.
        assert!(resolve_in(search_dirs, "../evil/pwn").is_none());
        // resolve() (the live, memoised path) rejects too.
        assert!(resolve("../evil/pwn").is_none());
    }

    // ── fallback menu shape ───────────────────────────────────────────

    #[test]
    fn build_enriched_with_actions_skips_synthetic_launch() {
        let e = parse_entry(COSMIC, "com.system76.CosmicFiles").unwrap();
        let m = build_enriched("com.system76.CosmicFiles", &e);
        assert_eq!(m.label, "Files");
        assert_eq!(m.service, SYNTHETIC_SERVICE);
        // Two top-level buttons: <App> and Window.
        assert_eq!(m.children.len(), 2);
        let app = &m.children[0];
        assert_eq!(app.label, "Files");
        // Declared action, separator, Quit — NO synthetic `xdg:` launch
        // leaf (it would duplicate the action's label).
        let labels: Vec<&str> = app.children.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, vec!["New Window", "", "Quit Files"]);
        assert_eq!(
            app.children[0].path,
            "xdg-action:com.system76.CosmicFiles:new-window"
        );
        assert_eq!(app.children[1].item_type, "separator");
        assert_eq!(app.children[2].path, "niri:close-window");
        // No item anywhere carries the default-launch `xdg:` path.
        assert!(app.children.iter().all(|c| !c.path.starts_with("xdg:")));
        // Window button reuses the niri synthetic submenu.
        assert_eq!(m.children[1].label, "Window");
    }

    #[test]
    fn build_enriched_without_actions_synthesises_launch() {
        // Obsidian-shape entry: a Name + Exec, zero `[Desktop Action]`s.
        let e = parse_entry(
            "[Desktop Entry]\nType=Application\nName=Obsidian\nExec=obsidian %u\n",
            "obsidian",
        )
        .unwrap();
        let m = build_enriched("obsidian", &e);
        let app = &m.children[0];
        let labels: Vec<&str> = app.children.iter().map(|c| c.label.as_str()).collect();
        // Default-launch leaf, separator, Quit.
        assert_eq!(labels, vec!["New Window", "", "Quit Obsidian"]);
        assert_eq!(app.children[0].path, "xdg:obsidian");
        assert_eq!(app.children[2].path, "niri:close-window");
    }

    #[test]
    fn fallback_menu_empty_app_id_is_none() {
        assert!(fallback_menu("").is_none());
        assert!(fallback_menu("   ").is_none());
    }

    #[test]
    fn fallback_menu_unresolved_app_id_uses_minimal_synthetic() {
        // An app_id that resolves to no .desktop entry still gets the
        // minimal identity fallback (app name + Window controls).
        let m = fallback_menu("org.nonexistent.WidgetFactory12345").unwrap();
        // crate::atspi::synthetic_menu shape: App + Window, no xdg leaves.
        assert_eq!(m.children.len(), 2);
        assert_eq!(m.children[1].label, "Window");
        fn assert_no_xdg(item: &MenuItem) {
            assert!(
                !item.path.starts_with("xdg:") && !item.path.starts_with("xdg-action:"),
                "minimal fallback must not contain xdg launch paths; got {}",
                item.path
            );
            for c in &item.children {
                assert_no_xdg(c);
            }
        }
        assert_no_xdg(&m);
    }

    #[test]
    fn fallback_menu_serialises_like_active_json_menu() {
        let e = parse_entry(COSMIC, "com.system76.CosmicFiles").unwrap();
        let m = build_enriched("com.system76.CosmicFiles", &e);
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["service"], SYNTHETIC_SERVICE);
        assert_eq!(json["children"][0]["children"][0]["type"], "standard");
        assert_eq!(
            json["children"][0]["children"][0]["path"],
            "xdg-action:com.system76.CosmicFiles:new-window"
        );
    }
}
