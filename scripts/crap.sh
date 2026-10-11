#!/usr/bin/env bash
# CRAP gate over the lcov `just cover` wrote. A function with no coverage
# record scores 0%, so a stale lcov is refused and test code is excluded.
# Silent on a pass; on a fail it names every function over 30.
# `--stale-reason` prints why the lcov is unusable, for wt-carry-lcov.sh.
set -euo pipefail
lcov=target/coverage/lcov.info
if [ ! -f "$lcov" ]; then
    stale="$lcov is missing"
else
    newer=$(find src tests -name '*.rs' -newer "$lcov" -print -quit)
    stale=${newer:+"$lcov predates $newer"}
fi
if [ "${1:-}" = --stale-reason ]; then
    [ -z "$stale" ] || echo "$stale"
    exit 0
fi
if [ -n "$stale" ]; then
    echo "just crap: $stale, run \`just cover\` first" >&2
    exit 1
fi
scripts/quiet.sh cargo crap --lcov "$lcov" --workspace \
    --exclude 'src/main.rs' --exclude 'tests/**' --exclude '**/*_tests.rs' \
    --min 30 --threshold 30 --fail-above
