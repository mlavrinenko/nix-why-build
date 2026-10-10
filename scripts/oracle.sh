#!/usr/bin/env bash
# Usage: oracle.sh INSTALLABLE [DIR]. Diffs this checkout's `--json`
# `to_build` with `nix build --dry-run` in a scratch store that holds .drv
# files but no outputs. Silent when they agree. A relative flake ref resolves
# against DIR. An empty `to_build` is refused: two empty lists always agree.
set -euo pipefail
installable=$1
case "$installable" in .*) installable="${2:-$PWD}/$installable" ;; esac
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
store="${XDG_CACHE_HOME:-$HOME/.cache}/nix-why-build/oracle-store"
mkdir -p "$store"
target/release/nix-why-build --json "$installable" > "$work/report.json"
top=$(jq -r .toplevel "$work/report.json")
jq -r '.to_build[].drv' "$work/report.json" | sort > "$work/ours"
if [ ! -s "$work/ours" ]; then
    echo "oracle: to_build is empty for $installable, so there is nothing to compare" >&2
    echo "hint: pick an installable that builds something, such as .#default" >&2
    exit 1
fi
nix copy --quiet --no-check-sigs --to "local?root=$store" "$top"
nix build --quiet --dry-run --store "local?root=$store" "$top^*" 2> "$work/plan"
awk '/will be built:$/ { on = 1; next } /^[^ ]/ { on = 0 } on { print $1 }' "$work/plan" \
    | sort > "$work/nix"
only_ours=$(comm -23 "$work/ours" "$work/nix")
only_nix=$(comm -13 "$work/ours" "$work/nix")
[ -z "$only_ours$only_nix" ] && exit 0
echo "only nix-why-build builds ($(grep -c . <<< "$only_ours" || true)):"
printf '%s\n' "$only_ours" | sed '/^$/d; s/^/  /'
echo "only nix builds ($(grep -c . <<< "$only_nix" || true)):"
printf '%s\n' "$only_nix" | sed '/^$/d; s/^/  /'
exit 1
