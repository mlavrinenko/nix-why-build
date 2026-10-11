#!/usr/bin/env bash
# List every available update for the workflows' actions, Cargo.toml and
# flake.lock, one line each: manager, dependency, current -> proposed.
#
# Renovate runs against this checkout with `--platform=local --dry-run=lookup`:
# it looks versions up and pushes, branches and opens nothing. Its JSON log is
# reduced to the `packageFiles with updates` record, which holds every
# manager's dependencies with their `updates`. The log is shown only when
# Renovate fails or warns, since a swallowed lookup error would read as
# "everything is current".
set -eo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
mkdir -p target/renovate
log=target/renovate/run.log

# github.com lookups: the token stays in this process's environment.
if [ -z "$GITHUB_COM_TOKEN" ] && command -v gh > /dev/null; then
    GITHUB_COM_TOKEN=$(gh auth token 2> /dev/null) || GITHUB_COM_TOKEN=
fi
export GITHUB_COM_TOKEN
export RENOVATE_CACHE_DIR="$root/target/renovate/cache"
export RENOVATE_BASE_DIR="$root/target/renovate/base"
export LOG_LEVEL=debug LOG_FORMAT=json

fail() {
    jq -r 'select(.level >= 40) | "\(.msg) \(del(.name, .hostname, .pid, .level, .logContext, .repository, .msg, .time, .v) | tojson)"' "$log" 2> /dev/null
    tail -n 5 "$log.err"
    exit 1
}

renovate --platform=local --dry-run=lookup > "$log" 2> "$log.err" || fail
jq -e -s 'any(.[]; .level >= 40)' "$log" > /dev/null && fail

jq -e -s 'any(.[]; .msg == "packageFiles with updates")' "$log" > /dev/null || {
    echo "renovate: no 'packageFiles with updates' record in $log" >&2
    exit 1
}

# A dependency is repeated per job; `unique` folds the copies. A digest-only
# update (flake inputs) shows short digests, anything else versions.
list=$(jq -r -s '
    def short: .[0:7];
    [ .[] | select(.msg == "packageFiles with updates") | .config | to_entries[]
      | .key as $manager
      | .value[].deps[] | select((.skipReason // "") == "")
      | . as $dep | .updates[]
      | ($dep.lockedVersion // $dep.currentValue // ($dep.currentDigest | short)) as $cur
      | (if .updateType == "digest" then ((.newDigest // "") | short) else (.newVersion // .newValue) end) as $new
      | "\(if $manager == "jsonata" then "flake.lock" else $manager end) \($dep.depName) \(if .updateType == "digest" then ($dep.currentDigest | short) else $cur end) -> \($new)"
    ] | unique[]' "$log")
[ -n "$list" ] || list="everything is current"
printf '%s\n' "$list"
