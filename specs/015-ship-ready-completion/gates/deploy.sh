#!/usr/bin/env bash
# deploy.sh — spec 015 SC-004 release gate.
#
# Walks the deploy-idempotence checklist (DI-001..DI-051) in
# read-only mode. The full DI-001..DI-029 protocol invokes
# scripts/release.sh twice against the same VERSION — that is
# expensive and writes through to the NixOS host. This gate is the
# *cheap* slice: artefact integrity (DI-040, DI-041) plus the
# HM-backup pre-clear smoke (DI-050, DI-051) plus a single
# version-parity confirmation.
#
# The full re-invocation idempotence smoke is left to scripts/
# release.sh itself — its stage-skip logic IS the production
# implementation of DI-020..DI-029, so re-running it twice is the
# integration test.
#
# Exit:
#   0 — PASS
#   1 — FAIL (artefact mismatch, stale .backup unhandled, version
#             drift)
#   2 — SKIP (no deployed bridge / not on a NixOS host)

set -euo pipefail

GATE_NAME="deploy-idempotence"

skip() {
    echo "[gate name=${GATE_NAME} result=SKIP reason=$1]" >&2
    echo "  remediation: $2" >&2
    exit 2
}

fail() {
    echo "[gate name=${GATE_NAME} result=FAIL $1]" >&2
    shift
    while [[ $# -gt 0 ]]; do echo "  $1" >&2; shift; done
    exit 1
}

pass() {
    echo "[gate name=${GATE_NAME} result=PASS $*]"
    exit 0
}

self_test() {
    # No external dependencies; exercise the .backup-detection regex.
    local sample="$HOME/.config/noctalia/foo.qml.backup"
    if [[ ! $sample =~ \.backup$ ]]; then
        echo "self-test FAIL: .backup regex broken" >&2
        exit 1
    fi
    echo "[gate name=${GATE_NAME} self-test=PASS]"
    exit 0
}

if [[ ${1:-} == "--self-test" ]]; then
    self_test
fi

# --- prerequisites ---

command -v noctalia-appmenu-bridge >/dev/null \
    || skip "missing-bridge-cli" "deploy noctalia-appmenu and ensure PATH includes it"
command -v systemctl >/dev/null \
    || skip "missing-systemctl" "gate requires a systemd user manager"

# --- DI-040: plugin symlink points at a Nix-store path for VERSION ---

bridge_version=$(noctalia-appmenu-bridge --version 2>/dev/null | awk '{print $NF}')
if [[ -z $bridge_version ]]; then
    fail "version-unreadable" \
         "noctalia-appmenu-bridge --version returned empty"
fi

plugin_root="${XDG_CONFIG_HOME:-$HOME/.config}/noctalia/plugins/noctalia-appmenu"
plugin_marker="${plugin_root}/AppmenuPopupWindow.qml"

if [[ ! -e $plugin_marker ]]; then
    skip "plugin-not-installed" \
         "expected ${plugin_marker}; run nh os switch (or scripts/release.sh nixos-deploy)"
fi

plugin_real=$(readlink -f "$plugin_marker")
if [[ $plugin_real != /nix/store/* ]]; then
    fail "plugin-not-store-linked plugin_real=${plugin_real}" \
         "DI-040 violation: plugin marker does not resolve into /nix/store"
fi

if [[ $plugin_real != *"-${bridge_version}"* ]] \
   && [[ $plugin_real != *"-${bridge_version}-"* ]] \
   && [[ $plugin_real != *"/${bridge_version}/"* ]]; then
    fail "plugin-version-mismatch plugin_real=${plugin_real} bridge_version=${bridge_version}" \
         "DI-040 violation: plugin store path does not name VERSION ${bridge_version}"
fi

# --- DI-041: bridge service ExecStart points at the same store path ---

exec_start=$(systemctl --user show noctalia-appmenu-bridge.service \
    --property=ExecStart --value 2>/dev/null | head -1)

if [[ -z $exec_start ]]; then
    skip "service-not-installed" \
         "noctalia-appmenu-bridge.service not present; install the HM module"
fi

# ExecStart format: `{ path=/nix/store/...; argv[]=...; ... }`
exec_path=$(awk -F'path=' '{print $2}' <<<"$exec_start" | awk -F';' '{print $1}')

if [[ -z $exec_path ]] || [[ $exec_path != /nix/store/* ]]; then
    fail "service-not-store-linked exec_path=${exec_path:-empty}" \
         "DI-041 violation: ExecStart path not /nix/store-resident"
fi

if [[ $exec_path != *"-${bridge_version}"* ]] \
   && [[ $exec_path != *"-${bridge_version}/"* ]] \
   && [[ $exec_path != *"/${bridge_version}/"* ]]; then
    fail "service-version-mismatch exec_path=${exec_path} bridge_version=${bridge_version}" \
         "DI-041 violation: ExecStart does not name VERSION ${bridge_version}"
fi

# --- DI-050: HM .backup pre-clear paths are clean ---
#
# We don't sweep here (that's release.sh's job). We assert that the
# paths the release skill is contracted to sweep are clean RIGHT NOW.
# If a .backup file accreted after the last release, the gate fails
# loudly so the operator knows the sweep coverage missed something.

sweep_roots=(
    "$HOME/.claude/plugins"
    "${XDG_CONFIG_HOME:-$HOME/.config}/git/hooks"
    "${XDG_CONFIG_HOME:-$HOME/.config}/noctalia"
    "${XDG_CONFIG_HOME:-$HOME/.config}/quickshell"
)

stale_backups=()
for root in "${sweep_roots[@]}"; do
    [[ -d $root ]] || continue
    while IFS= read -r -d '' f; do
        stale_backups+=("$f")
    done < <(find "$root" -maxdepth 6 -type f -name "*.backup" -print0 2>/dev/null)
done

if [[ ${#stale_backups[@]} -gt 0 ]]; then
    fail "stale-backups=${#stale_backups[@]}" \
         "DI-050 violation: .backup files present under HM-sweep roots" \
         "${stale_backups[@]}"
fi

# --- DI-029 surrogate: proxy bus name present ---

if command -v busctl >/dev/null 2>&1; then
    if ! busctl --user list 2>/dev/null | grep -q '^org\.noctalia\.AppMenu\b'; then
        fail "proxy-bus-missing" \
             "expected org.noctalia.AppMenu on session bus; bridge may have crashed"
    fi
fi

pass "version=${bridge_version} plugin_store=${plugin_real##*/} service_store=${exec_path##*/} stale_backups=0"
