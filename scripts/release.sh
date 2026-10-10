#!/usr/bin/env bash
# Usage: release.sh VERSION [--dry-run]. Runs every check that needs no push,
# pushes main, waits for ci.yml on that commit and tags it only when green.
# Each refusal prints `error:` and a `hint:` with the way out.
set -euo pipefail
version=$1
tag="v$version"
dry_run=false
refuse() { echo "error: $1" >&2; shift; for h in "$@"; do echo "hint: $h" >&2; done; exit 1; }
for flag in "${@:2}"; do
    case "$flag" in
        --dry-run) dry_run=true ;;
        *) refuse "unknown flag '$flag'" "just release VERSION [--dry-run]" ;;
    esac
done
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] \
    || refuse "'$version' is not a semver version" "give it without the v: just release 1.2.3"
cargo_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ "$version" = "$cargo_version" ] \
    || refuse "requested '$tag' but Cargo.toml is '$cargo_version'" "bump Cargo.toml to '$version' and commit it first"
for tool in git cargo curl gh; do
    command -v "$tool" >/dev/null \
        || refuse "'$tool' is not on PATH" "run inside the dev shell"
done
gh auth status >/dev/null 2>&1 || refuse "gh is not logged in" "gh auth login"
crate=$(sed -n 's/^name = "\(.*\)"/\1/p' Cargo.toml | head -1)
repo_url=$(sed -n 's/^repository = "\(.*\)"/\1/p' Cargo.toml | head -1)
workflows=(.github/workflows/release*.yml)
workflow=$(basename "${workflows[0]}")
[ -z "$(git status --porcelain)" ] \
    || refuse "the working tree has uncommitted changes" "commit or discard them, then re-run"
branch=$(git symbolic-ref --short -q HEAD || true)
[ "$branch" = main ] || refuse "HEAD is on '${branch:-a detached commit}', not main" "git switch main"
git fetch --quiet origin main || refuse "can't fetch 'origin main'" "check the network and the origin remote"
git merge-base --is-ancestor origin/main HEAD \
    || refuse "origin/main has commits that main lacks" "git pull --ff-only origin main, then re-run"
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null \
    || git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null; then
    refuse "'$tag' already exists" "a pushed tag never moves; finish a failed release with: gh workflow run $workflow -f tag=$tag"
fi
status=$(curl -sS -o /dev/null -w '%{http_code}' -A "$repo_url" \
    "https://crates.io/api/v1/crates/$crate/$version" || true)
case "$status" in
    404) ;;
    200) refuse "'$crate $version' is already on crates.io" "a published version is never re-published; bump to the next version" ;;
    *) refuse "crates.io answered HTTP '${status:-nothing}' for '$crate $version'" "retry once crates.io is reachable" ;;
esac
grep -Eq "^## \[${version//./\\.}\] - [0-9]{4}-[0-9]{2}-[0-9]{2}" CHANGELOG.md \
    || refuse "CHANGELOG.md has no dated section for '$version'" "move [Unreleased] under '## [$version] - $(date +%F)' and commit"
mmz --is-fresh --tag gate || refuse "the gates have not passed on this tree" "run just check"
cargo publish --dry-run --locked --quiet || refuse "cargo publish --dry-run failed" "fix the packaging error above"
head=$(git rev-parse HEAD)
short=$(git rev-parse --short HEAD)
ahead=$(git rev-list --count origin/main..HEAD)
if [ "$ahead" -gt 0 ]; then
    plan="push $ahead commit(s) to origin main ('$(git rev-parse --short origin/main)'..'$short')"
else
    plan="push nothing (origin main is already '$short')"
fi
if $dry_run; then
    echo "dry run: every check passed"
    echo "would $plan, wait for ci.yml on '$short', then tag and push '$tag'"
    exit 0
fi
if [ "$ahead" -gt 0 ]; then
    git push --quiet origin "$head:refs/heads/main" \
        || refuse "origin rejected the push of main" "git pull --ff-only origin main, then re-run"
fi
run=""
deadline=$((SECONDS + 300))
while :; do
    run=$(gh run list --commit "$head" --workflow ci.yml --limit 1 \
        --json databaseId --jq '.[0].databaseId // empty') \
        || refuse "can't list ci.yml runs for '$short'" "check gh auth status and that .github/workflows/ci.yml exists"
    [ -n "$run" ] && break
    [ "$SECONDS" -lt "$deadline" ] \
        || refuse "no ci.yml run showed up for '$short' in 5 minutes, so '$tag' was not made" "main is pushed; once ci.yml is green on it, re-run just release $version"
    sleep 5
done
echo "waiting for ci.yml run '$run' on '$short'"
if ! gh run watch "$run" --exit-status --compact --interval 15 >/dev/null; then
    conclusion=$(gh run view "$run" --json conclusion --jq .conclusion 2>/dev/null || true)
    refuse "ci.yml ended '${conclusion:-unknown}' on '$short', so '$tag' was not made" \
        "see why: gh run view $run --log-failed" \
        "fix main and re-run just release $version, or retry a flake with: gh run rerun $run --failed"
fi
git tag -a "$tag" -m "$tag" "$head"
git push --quiet origin "refs/tags/$tag" \
    || { git tag -d "$tag" >/dev/null; refuse "origin rejected the push of '$tag'" "re-run just release $version"; }
echo "pushed '$tag'; follow it with: gh run list --workflow $workflow --limit 1"
