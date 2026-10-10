#!/usr/bin/env bash
# Run a command and print its output only when it fails, like `chronic`.
out=$("$@" 2>&1) || { status=$?; [ -z "$out" ] || printf '%s\n' "$out"; exit "$status"; }
