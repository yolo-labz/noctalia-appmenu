# noctalia-appmenu task runner — `just <target>` from anywhere in the repo.
# All targets assume the dev shell is loaded (direnv) or use `nix develop -c`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list --unsorted

# ---------- bridge ----------

bridge-build:
    cd bridge && cargo build --all-features

bridge-test:
    cd bridge && cargo test --all-features --no-fail-fast

bridge-cov:
    cd bridge && cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

bridge-clippy:
    cd bridge && cargo clippy --all-features --all-targets -- -D warnings

bridge-fmt:
    cd bridge && cargo fmt

bridge-run-fg *FLAGS:
    cd bridge && RUST_LOG=noctalia_appmenu_bridge=debug cargo run -- --foreground {{FLAGS}}

# ---------- plugin ----------

plugin-lint:
    qmllint plugin/BarWidget.qml plugin/components/*.qml

plugin-install-local:
    mkdir -p ~/.config/noctalia/plugins/noctalia-appmenu
    cp -r plugin/* ~/.config/noctalia/plugins/noctalia-appmenu/

# ---------- nix ----------

flake-check:
    nix flake check --print-build-logs

flake-fmt:
    alejandra --check .

flake-fmt-fix:
    alejandra .

build:
    nix build .#noctalia-appmenu-bridge .#noctalia-appmenu-plugin

# ---------- integration ----------

fake-registrar:
    python3 tools/fake-registrar/registrar.py

integration:
    cd tests/integration && bash ./run.sh

# ---------- governance ----------

changelog:
    git-cliff -o CHANGELOG.md

actionlint:
    actionlint -color

zizmor:
    zizmor --persona=auditor .github/workflows/

precommit:
    lefthook run pre-commit

# ---------- local prechew (mirrors CI) ----------

# Install lefthook hooks into .git/hooks/. Run once after cloning.
lefthook-install:
    lefthook install

# The full pre-push pipeline, on demand. Mirrors the CI matrix locally
# in parallel — see lefthook.yml's pre-push: section. ADR-0014.
shadow-ci:
    lefthook run pre-push

# Auto-fix everything that has an auto-fix path. Run before shadow-ci
# to keep the loop tight.
fix:
    cd bridge && cargo fmt --all
    alejandra .
    qmlformat -i plugin/BarWidget.qml plugin/components/*.qml
    typos --config typos.toml --write-changes || true

# Same as shadow-ci but also runs the heavyweight nix flake build.
ci-fast:
    just shadow-ci
    nix build .#noctalia-appmenu-bridge --print-build-logs

# ---------- release ----------

# Refuses to tag if the working tree is dirty or HEAD is not on origin/main.
release-tag VERSION:
    @if ! git diff-index --quiet HEAD --; then echo "dirty tree"; exit 1; fi
    @if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then echo "HEAD != origin/main"; exit 1; fi
    git tag -s -m "v{{VERSION}}" v{{VERSION}}
    @echo "Now: git push origin v{{VERSION}}"
