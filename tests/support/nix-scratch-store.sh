#!/usr/bin/env bash
# nextest setup script: the tests that run the real `nix` get a scratch store
# through NIX_REMOTE, so the .drv files they instantiate never reach /nix/store.
# See src/nix_tests.rs for why this and not `--read-only`.
set -eo pipefail
root="${CARGO_TARGET_DIR:-$PWD/target}/nix-scratch"
mkdir -p "$root"
store="local?root=$(cd "$root" && pwd)"
# Create the store once, so parallel tests do not race to initialize it. Its
# report is noise unless the store cannot be opened.
report=$(NIX_REMOTE="$store" nix store info 2>&1) || {
    echo "$report" >&2
    exit 1
}
echo "NIX_REMOTE=$store" >> "$NEXTEST_ENV"
