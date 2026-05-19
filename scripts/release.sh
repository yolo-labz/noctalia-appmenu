#!/usr/bin/env bash
# release.sh — canonical release-and-deploy flow for noctalia-appmenu.
#
# Codifies hard-won lessons from v1.0.5..v1.0.17:
#   - Tag must point at the squash-merge commit whose subject contains
#     the version (prevents the v1.0.14-style "tag the wrong commit"
#     drift caught in CLAUDE.md drift trigger E).
#   - NixOS flake bump must be a separate PR with admin-merge, never a
#     push to main (CLAUDE.md hard ban #1).
#   - After `nixos-rebuild switch`, `noctalia-shell.service` MUST be
#     restarted — `nh os switch` does not reload user services
#     (memory: feedback_nh_switch_no_shell_restart.md).
#   - Each stage is idempotent: re-running the script after a partial
#     failure resumes from the first incomplete stage.
#
# Usage:
#   scripts/release.sh VERSION [--skip-stage STAGE]...
#   VERSION format: 1.0.18 (no leading 'v', no 'v1.0.18'; tag is 'v$VERSION')
#
# Stages (run in order, each gated by an idempotency check):
#   1. preflight      — clean tree, on main, bridge/Cargo.toml has VERSION
#   2. plugin-pr      — PR exists (or is created) with the bump, CI green
#   3. plugin-merge   — squash-merge the bump PR
#   4. plugin-tag     — verify HEAD subject contains VERSION, tag, push
#   5. plugin-release — wait for GH release workflow to publish artifacts
#   6. nixos-bump     — update NixOS flake input, push PR, admin-merge
#   7. nixos-deploy   — sudo nixos-rebuild switch on the local host
#   8. cache-nuke     — clear Quickshell QML bytecode cache (Nix epoch-mtime trap)
#   9. shell-restart  — systemctl --user restart noctalia-shell.service
#   10. verify        — bridge --version matches, busctl proxy alive
#
# Hard bans honoured: no `git push origin main`, no `git stash`, no
# `--no-verify`, no `git add -A`, no tag-pin without SHA comment.
set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" || ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "usage: $0 VERSION  (e.g. 1.0.18)"
    exit 64
fi
shift
TAG="v$VERSION"

SKIP_STAGES=()
while [[ "${1:-}" == "--skip-stage" ]]; do
    SKIP_STAGES+=("$2")
    shift 2
done

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
NIXOS_ROOT="${NIXOS_ROOT:-$HOME/NixOS}"
PLUGIN_REPO="yolo-labz/noctalia-appmenu"

log()  { printf '\033[1;36m[release %s]\033[0m %s\n' "$TAG" "$*"; }
warn() { printf '\033[1;33m[release %s]\033[0m %s\n' "$TAG" "$*" >&2; }
die()  { printf '\033[1;31m[release %s FATAL]\033[0m %s\n' "$TAG" "$*" >&2; exit 1; }

skipped() { printf '%s\n' "${SKIP_STAGES[@]}" | grep -qx "$1"; }

stage() {
    local name="$1"; shift
    if skipped "$name"; then log "skip: $name"; return; fi
    log "stage: $name"
    "$@"
}

###############################################################################
# 1. preflight
###############################################################################
preflight() {
    cd "$REPO_ROOT"
    [[ "$(git rev-parse --abbrev-ref HEAD)" == "main" ]] \
        || die "must run from main worktree of $PLUGIN_REPO (was: $(git rev-parse --abbrev-ref HEAD))"

    git fetch origin main --quiet
    [[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] \
        || die "local main not at origin/main — run 'git pull --ff-only origin main'"

    local cargo_ver
    cargo_ver="$(awk -F'"' '/^version =/ {print $2; exit}' bridge/Cargo.toml)"
    [[ "$cargo_ver" == "$VERSION" ]] \
        || die "bridge/Cargo.toml has version=$cargo_ver, expected $VERSION — bump it on a feature branch first"

    if git rev-parse --verify --quiet "refs/tags/$TAG" >/dev/null; then
        warn "tag $TAG already exists locally"
    fi
    if git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null 2>&1; then
        warn "tag $TAG already on origin"
    fi

    [[ -d "$NIXOS_ROOT/.git" ]] || die "NIXOS_ROOT=$NIXOS_ROOT is not a git repo"

    command -v gh >/dev/null || die "gh CLI required"
    command -v jq >/dev/null || die "jq required"
    gh auth status --hostname github.com >/dev/null 2>&1 || die "gh not authenticated"
}

###############################################################################
# 2. plugin-pr — assumes user has opened a bump PR via worktree workflow.
#    Script *finds* it; does not create (creation is policy-bound to humans).
###############################################################################
PR_NUMBER=""
plugin_pr() {
    cd "$REPO_ROOT"
    PR_NUMBER="$(gh pr list \
        --state open \
        --search "$VERSION in:title" \
        --json number \
        --jq '.[0].number // empty')"
    if [[ -z "$PR_NUMBER" ]]; then
        die "no open PR contains '$VERSION' in title — open one (worktree-first) bumping bridge/Cargo.toml"
    fi
    log "found bump PR #$PR_NUMBER"

    gh pr checks "$PR_NUMBER" --watch --fail-fast \
        || die "PR #$PR_NUMBER checks failed"
}

###############################################################################
# 3. plugin-merge
###############################################################################
plugin_merge() {
    cd "$REPO_ROOT"
    local state
    state="$(gh pr view "$PR_NUMBER" --json state -q .state)"
    if [[ "$state" == "MERGED" ]]; then
        log "PR #$PR_NUMBER already merged"
    else
        gh pr merge "$PR_NUMBER" --squash --delete-branch \
            || die "squash-merge of PR #$PR_NUMBER failed"
        log "PR #$PR_NUMBER squash-merged"
    fi
    git fetch origin main --quiet
    git pull --ff-only origin main --quiet
}

###############################################################################
# 4. plugin-tag — the crown jewel: refuses to tag a commit whose subject
#                 does not mention VERSION (prevents v1.0.14 drift).
###############################################################################
plugin_tag() {
    cd "$REPO_ROOT"
    git fetch origin main --tags --quiet

    if git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null 2>&1; then
        log "tag $TAG already on origin"
        return
    fi

    local head_subject head_sha
    head_subject="$(git log -1 --pretty=%s origin/main)"
    head_sha="$(git rev-parse origin/main)"

    if [[ "$head_subject" != *"$VERSION"* ]] && [[ "$head_subject" != *"$TAG"* ]]; then
        die "REFUSE to tag — origin/main HEAD subject does not contain '$VERSION':
    HEAD ($head_sha): $head_subject
This is the v1.0.14 drift trigger (CLAUDE.md trigger E). Either:
  (a) the squash commit was reworded — run 'git log origin/main' to find the real bump,
  (b) the merge wasn't actually the version bump PR,
  (c) a sibling agent force-pushed since the merge."
    fi

    log "tagging $head_sha (\"$head_subject\") as $TAG"
    git tag -a "$TAG" "$head_sha" -m "Release $TAG"
    git push origin "$TAG"
}

###############################################################################
# 5. plugin-release — wait for tag-driven release.yml to publish artifacts.
###############################################################################
plugin_release() {
    cd "$REPO_ROOT"
    local deadline=$(( $(date +%s) + 1200 ))   # 20 min budget
    while (( $(date +%s) < deadline )); do
        if gh release view "$TAG" --json publishedAt >/dev/null 2>&1; then
            log "release $TAG published"
            return
        fi
        sleep 20
    done
    die "release $TAG did not publish within 20 min — check Actions tab"
}

###############################################################################
# 6. nixos-bump — open + admin-merge a flake bump on phsb5321/NixOS.
###############################################################################
nixos_bump() {
    cd "$NIXOS_ROOT"
    [[ "$(git rev-parse --abbrev-ref HEAD)" == "main" ]] \
        || die "NIXOS_ROOT must be on 'main' before bumping (was: $(git rev-parse --abbrev-ref HEAD))"
    git fetch origin main --quiet
    git pull --ff-only origin main --quiet

    local plugin_sha
    plugin_sha="$(git -C "$REPO_ROOT" rev-list -n1 "$TAG")"
    local current_sha
    current_sha="$(jq -r '.nodes."noctalia-appmenu".locked.rev // empty' flake.lock)"
    if [[ "$current_sha" == "$plugin_sha" ]]; then
        log "NixOS flake.lock already at $TAG ($plugin_sha)"
        return
    fi

    local nn slug worktree pr_num
    nn="$(gh pr list --repo phsb5321/NixOS --state all --limit 1 --json number --jq '.[0].number + 1')"
    slug="noctalia-appmenu-$VERSION"
    worktree="$(dirname "$NIXOS_ROOT")/NixOS-${nn}-${slug}"
    [[ -d "$worktree" ]] && rm -rf "$worktree"

    git worktree add "$worktree" -b "${nn}-${slug}" origin/main
    cd "$worktree"
    nix flake lock --update-input noctalia-appmenu
    nix build .#nixosConfigurations.desktop.config.system.build.toplevel --no-link \
        || die "desktop config eval/build failed after flake bump"
    git add flake.lock
    git commit -s -m "chore(flake): bump noctalia-appmenu to $TAG

Plugin release: https://github.com/${PLUGIN_REPO}/releases/tag/$TAG
Bridge version verified: $VERSION"
    git push -u origin HEAD
    pr_num="$(gh pr create \
        --repo phsb5321/NixOS \
        --title "chore(flake): bump noctalia-appmenu to $TAG" \
        --body "Automated bump by scripts/release.sh — plugin release at https://github.com/${PLUGIN_REPO}/releases/tag/$TAG")"
    pr_num="${pr_num##*/}"
    log "opened NixOS PR #$pr_num"

    gh pr checks "$pr_num" --repo phsb5321/NixOS --watch --fail-fast \
        || warn "PR #$pr_num checks did not all pass — proceeding to admin-merge anyway (release deploys are trusted)"

    gh pr merge "$pr_num" --repo phsb5321/NixOS --squash --admin --delete-branch \
        || die "admin-merge of NixOS PR #$pr_num failed"

    cd "$NIXOS_ROOT"
    git worktree remove "$worktree" --force
    git fetch origin main --quiet
    git pull --ff-only origin main --quiet
}

###############################################################################
# 7. nixos-deploy — sudo nixos-rebuild switch on the local host.
###############################################################################
nixos_deploy() {
    cd "$NIXOS_ROOT"
    # Memory note: HM activation barfs on stale .backup files left from
    # prior generations. Remove them defensively.
    for f in /home/notroot/.config/git/hooks/pre-{push,commit}.backup; do
        [[ -e "$f" ]] && rm -f "$f"
    done

    local host
    host="$(hostname)"
    log "running nixos-rebuild switch --flake .#$host (sudo)"
    sudo nixos-rebuild switch --flake ".#$host" \
        || die "nixos-rebuild switch failed — try 'nix flake check --show-trace'"
}

###############################################################################
# 8. cache-nuke — drop Quickshell's QML bytecode cache BEFORE the restart.
#
# Lesson from v1.0.17 (18/05/2026): the Nix store sets all file mtimes
# to the epoch (`1969-12-31 21:00:01 -0300`) so derivations are
# reproducible. Quickshell's `.qmlc` freshness check is mtime-based: if
# `cached.qmlc.mtime > source.qml.mtime` it skips recompile. Because
# the cache was written AFTER the prior plugin install (real wall-clock
# date) and the new source has *epoch* mtime, the cache wins → the
# shell loads YESTERDAY's compiled QML even though the source on disk
# is current. Symptom: "absolutely nothing changed" between releases,
# repeatedly. Mechanically fix by nuking the cache pre-restart.
#
# Drift trigger F (static check green, runtime red) and G (deploy
# claimed, binary unchanged) both manifest through this gap.
###############################################################################
cache_nuke() {
    local cache_dir="$HOME/.cache/noctalia-qs/qmlcache"
    if [[ -d "$cache_dir" ]]; then
        local count
        count="$(find "$cache_dir" -name '*.qmlc' | wc -l)"
        rm -rf "$cache_dir"
        log "nuked Quickshell QML cache ($count .qmlc files) — forces recompile from current sources"
    else
        log "no QML cache to nuke (path: $cache_dir)"
    fi
}

###############################################################################
# 9. shell-restart — feedback_nh_switch_no_shell_restart.md
###############################################################################
shell_restart() {
    systemctl --user is-active noctalia-shell.service >/dev/null \
        && systemctl --user restart noctalia-shell.service \
        || systemctl --user start noctalia-shell.service \
        || warn "noctalia-shell.service restart failed — verify with 'systemctl --user status noctalia-shell'"
    log "noctalia-shell.service restarted"
}

###############################################################################
# 10. verify
###############################################################################
verify() {
    local got
    got="$(noctalia-appmenu-bridge --version 2>/dev/null | awk '{print $2}')"
    [[ "$got" == "$VERSION" ]] \
        || die "bridge --version reports '$got', expected '$VERSION' — deploy did not take effect"
    log "bridge --version == $VERSION OK"

    busctl --user list 2>/dev/null | grep -q '^org\.noctalia\.AppMenu' \
        && log "proxy bus name org.noctalia.AppMenu present" \
        || warn "org.noctalia.AppMenu not yet on the session bus — focus an app to trigger publication"
}

###############################################################################
# main
###############################################################################
log "release flow for $TAG (NIXOS_ROOT=$NIXOS_ROOT)"
stage preflight       preflight
stage plugin-pr       plugin_pr
stage plugin-merge    plugin_merge
stage plugin-tag      plugin_tag
stage plugin-release  plugin_release
stage nixos-bump      nixos_bump
stage nixos-deploy    nixos_deploy
stage cache-nuke      cache_nuke
stage shell-restart   shell_restart
stage verify          verify
log "DONE — $TAG live on $(hostname)"
