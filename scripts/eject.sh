#!/usr/bin/env bash
# Eject inline tests from Rust files at PCT% (default 90) of the linecop
# limit. `linecop --baseline` exits 1 to flag files, which is what we want
# here; only 2, a broken run, fails.
set -euo pipefail
rc=0
paths=$(linecop --baseline "${1:-90}" --format paths) || rc=$?
[ "$rc" -le 1 ] || exit "$rc"
rust=$(printf '%s\n' "$paths" | grep '\.rs$' || true)
[ -z "$rust" ] || printf '%s\n' "$rust" | ejectest apply src --files-from - --lenient
