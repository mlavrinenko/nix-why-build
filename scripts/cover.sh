#!/usr/bin/env bash
# Coverage floor (95% of lines) on the nextest runner `just test` uses.
# `clean` first: without it a long-lived checkout keeps scoring binaries
# that no longer exist. Silent on a pass; on a fail it prints why.
set -euo pipefail
ignore='src/main\.rs|/registry/src/'
mkdir -p target/coverage
scripts/quiet.sh cargo llvm-cov clean --workspace
scripts/quiet.sh cargo llvm-cov nextest --workspace --no-report
scripts/quiet.sh cargo llvm-cov report --ignore-filename-regex "$ignore" \
    --lcov --output-path target/coverage/lcov.info
scripts/quiet.sh cargo llvm-cov report --ignore-filename-regex "$ignore" --fail-under-lines 95
