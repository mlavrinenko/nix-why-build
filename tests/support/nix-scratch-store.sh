#!/usr/bin/env bash
# nextest setup script: the tests that run the real `nix` get an in-memory
# store through NIX_REMOTE, so the .drv files they instantiate never reach
# /nix/store, and nothing read-only lands under target/ to stop `cargo clean`.
# See src/nix_tests.rs for why this and not `--read-only`.
set -eo pipefail
echo "NIX_REMOTE=dummy://?read-only=false" >> "$NEXTEST_ENV"
