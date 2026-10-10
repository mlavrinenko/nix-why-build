set quiet := true

# Shell for every non-shebang recipe line. `-e` aborts the line at its first
# failing command, `-o pipefail` makes a failure anywhere in a pipeline count
# (neither holds in just's default `sh -cu`), so recipes need no per-recipe
# `set -eo pipefail` preamble. No `-u`: recipe bodies interpolate `{{ }}`
# values and read optional env overrides, where an unset name should expand
# empty, not abort. Note that just runs each recipe LINE in a fresh shell —
# a recipe needing one shell across several lines (a variable read back
# later, a `trap`) still carries its own `#!/usr/bin/env bash` shebang, and
# `set shell` does not apply to those.
set shell := ["bash", "-eo", "pipefail", "-c"]

# Keep `#` lines inside a plain recipe in the justfile instead of handing each
# one to its own throwaway shell. Shebang recipe bodies are untouched.
set ignore-comments := true

# List available recipes
default:
    @just --list

# clippy-fix before fmt: a clippy autofix can land unformatted, so format after
# it, and the `check` that follows (which re-runs fmt-check) stays green.
# Run fixes, then other checks
fix-check: eject clippy-fix fmt check

# Run all checks in parallel (fmt + clippy + tests + unused deps + file size +
# drift + copy-paste). Each arm is `just mmz <subgate>` — mmz wraps it, so a
# no-op `just check` re-run skips every arm whose declared inputs are unchanged,
# and `mmz --is-fresh --tag gate` asserts they all passed without running any.
# Per-rule scopes and the `gate` tag live in .mmz/config.yaml.
[parallel]
check: (mmz "fmt-check") (mmz "clippy") (mmz "test") (mmz "machete") (mmz "check-file-size") (mmz "check-dry") (mmz "outdatty-check")

# Wrap a just recipe in mmz: `just mmz fmt-check` runs `mmz just fmt-check`. The
# double hop makes mmz the cache identity (rule name `just <recipe>` in
# .mmz/config.yaml), uniform across every sub-gate of `just check`. `just test
# <filter>` (a developer query, no `mmz`) bypasses this recipe and runs the
# suite directly with ARGS passed to both passes — and the `just test` rule is
# `match: exact`, so even a stray `just mmz test <filter>` cannot record the
# full-suite identity off a filtered subset.
#
# `[no-exit-message]`: on a miss mmz prints its own reason, and just's
# `error: Recipe 'mmz' failed` line above the real output only buries it.
[no-exit-message]
mmz *ARGS:
    mmz just {{ ARGS }}

# Show each memoized rule's freshness — i.e. which arms of `just check` would
# actually re-run, and whether the pre-commit hook's `mmz --is-fresh --tag
# gate` would pass right now. Reads mmz's records; runs no gate.
status:
    mmz --status

# Fail if a source changed without its dependents being re-confirmed
outdatty-check:
    outdatty check

# Re-confirm dependency groups: record current hashes into outdatty.lock
outdatty-update:
    outdatty update

# Run tests only. The gate runs the suite through cargo-nextest — one process
# per test, so a panicking or hanging test is isolated — with `--status-level
# fail`: a green run prints only the run header and the summary line, a red one
# prints every failure's full output. nextest does not run doctests, so a second
# pass covers them: a project with none pays a fast `running 0 tests` no-op.
#
# `[positional-arguments]` + `"$@"`: ARGS still reach both passes (a plain
# substring filter works for both), but interpolating the variadic would
# re-split a filter containing spaces and re-glob a wildcard.
[positional-arguments]
test *ARGS:
    cargo nextest run --workspace --status-level fail "$@"
    cargo test --workspace --doc "$@"

# Run clippy only
clippy:
    cargo clippy --workspace --all-targets -q -- -D warnings

# Auto-fix clippy warnings (allow-dirty/-staged: fix-check runs pre-commit, tree is dirty).
clippy-fix:
    cargo clippy --fix --workspace --all-targets --allow-dirty --allow-staged -- -D warnings

# Build the project
build *args:
    cargo build --workspace -q {{ args }}

# Paths kept out of the coverage report, shared by both report passes below.
# main.rs is excluded — keep it thin; `just crap` excludes it too. The
# /registry/src/ ignore keeps local and CI reporting the same set when
# CARGO_HOME is relocated. A file-scope variable, not a shell one: `cover` has
# no shebang, so each of its lines runs in a fresh shell.
cover_ignore := 'src/main\.rs|/registry/src/'

# Run coverage with cargo-llvm-cov, instrumenting the SAME runner as `just
# test` (`cargo llvm-cov nextest`). Process-per-test and shared-process are
# different execution models: a gate that tests one while coverage exercises
# the other can pass a test that fails under the other runner with nothing
# saying so. llvm-cov, unlike tarpaulin, merges the profraw of child
# processes, so subprocess-based CLI e2e tests actually attribute to the code
# they exercise. Instrument once (--no-report), then render both reports from
# that profdata.
#
# It carries no `--status-level fail`, and that absence is deliberate:
# `cargo llvm-cov nextest` accepts the flag and ignores it. The setting lives in
# `.config/nextest.toml` instead, where both runners read it — a flag that is
# accepted and ignored is worse than one that is absent, because the recipe then
# claims something the output disproves.
#
# `clean --workspace` first, and it is not tidiness. The `--no-report` +
# separate `report` split does NOT discard earlier profraw, so a long-lived
# checkout merges coverage maps from binaries that no longer exist. Measured on
# a real project: a 106-line file scored as **357 instrumented lines at 21.65%**
# — the pre-split version, still counted weeks after it stopped existing — and a
# workspace headline of 87.58% where the tree was actually 93.45%. It fails
# toward pessimism, so a gate with a floor never complains; what it does instead
# is make every fresh worktree disagree with the main checkout.
cover:
    mkdir -p target/coverage
    cargo llvm-cov clean --workspace
    cargo llvm-cov nextest --workspace --no-report
    cargo llvm-cov report --ignore-filename-regex '{{ cover_ignore }}' \
        --lcov --output-path target/coverage/lcov.info
    cargo llvm-cov report --ignore-filename-regex '{{ cover_ignore }}' --fail-under-lines 70

# Gate complex, undertested functions via CRAP metric. Needs lcov from `just
# cover` first, and REFUSES a stale one rather than scoring it — see
# `_lcov-stale-reason` below. Threshold 30 is a sane greenfield default; tune
# per repo. `--exclude main.rs` is harmless for libs (where the file doesn't
# exist).
#
# Everything else here is one bug in three disguises. cargo-crap's default
# `--missing pessimistic` scores a function with NO coverage record as 0%, and
# three kinds of file have no record:
#
#   - Test code. llvm-cov instruments the code under test, not the suite, so
#     nothing under `tests/` and no ejected `*_tests.rs` has a record. Without
#     these excludes the gate grades your test helpers and can fail on a
#     branchy TEST — measured elsewhere at 101 files scored against a
#     measurement nobody took.
#   - Anything the report was never asked to cover. Keep `--workspace` and the
#     `-p`/exclude set here in step with what `cover` above renders: a member
#     built for another target (a wasm client, say) is uninstrumented, so
#     `--workspace` grades a whole crate at 0%.
#   - Every file newer than the lcov itself, which is what `_lcov-stale-reason`
#     catches.
#
# `--min 30` matched to `--threshold 30` is what makes a green run quiet: the
# table prints only what is at or above the cutoff, so there is nothing to say
# when nothing is wrong, and a red run still names every offender. Measured
# elsewhere at **3532 lines to report success** — every analyzed function,
# passing ones included — against 10 with the flag.
crap:
    #!/usr/bin/env bash
    set -eo pipefail
    reason=$(just _lcov-stale-reason)
    if [ -n "$reason" ]; then
        echo "just crap: $reason — run \`just cover\` first" >&2
        echo "  (a source the lcov never saw scores as 0% covered, not as unknown)" >&2
        exit 1
    fi
    cargo crap --lcov target/coverage/lcov.info --workspace \
        --exclude 'src/main.rs' --exclude 'tests/**' --exclude '**/*_tests.rs' \
        --min 30 --threshold 30 --fail-above

# Is target/coverage/lcov.info usable against the tree in front of it? Empty
# stdout means yes; any other line names why not. It ALWAYS exits 0 — the exit
# status is not the channel, so a caller that treats a reason as ordinary does
# not also get just's own `error: Recipe ... failed` line. One recipe rather
# than a copied `find`, because two staleness tests that could disagree are
# worse than none.
#
# The set is what `cover` measures plus what exercises it: `src` is rendered,
# `tests` is the instrument, so a changed test moves the coverage of files it
# never edited. Extend it when a workspace member's sources land outside `src/`.
#
# Why a refusal and not a `cover` dependency: that recipe is minutes of
# instrumented CPU and this gate is seconds, so someone iterating on one
# function's tests must be able to re-read the existing report. The refusal says
# the number would be a lie and leaves paying for a true one to the caller.
_lcov-stale-reason:
    #!/usr/bin/env bash
    set -eo pipefail
    lcov=target/coverage/lcov.info
    if [ ! -f "$lcov" ]; then
        echo "$lcov is missing"
        exit 0
    fi
    newer=$(find src tests -name '*.rs' -newer "$lcov" -print -quit)
    if [ -n "$newer" ]; then
        echo "$lcov predates $newer"
    fi

# Format code
fmt:
    cargo fmt --all

# Format check (CI-friendly)
fmt-check:
    cargo fmt --all -- --check

# Check for unused dependencies
machete:
    cargo machete

# Count tests across the workspace. `cargo nextest list` reads the suite
# without running it, so the count no longer depends on the runner's output
# format — the old summary scrape ran the whole suite just to count it.
# Ignored tests are excluded, like nextest's own summary. Doctests are not
# counted: only `--json` would report them, and a count is a dev aid, not a
# gate.
count-tests:
    cargo nextest list --workspace --message-format json \
        | jq -r '[.["rust-suites"][].testcases[] | select(.ignored == false)] | "\(length) tests"'

# Show top 20 files by line count. `awk NR<=20` and not `head -20`: head exits
# after its 20th line, which SIGPIPEs sort once the listing outgrows the pipe
# buffer, and pipefail would report that 141 as a recipe failure.
file-sizes:
    find . -type f \( -name '*.rs' -o -name '*.md' \) ! -path './target/*' -exec wc -l {} + | sort -rn | awk 'NR <= 20'

# Eject inline tests from Rust files nearing the linecop limit, so they stay
# under it without losing the inline-test workflow. Runs as part of `fix-check`.
# linecop's `--baseline` exit is informational: it exits 1 merely to FLAG files
# at/over the threshold, not because it failed to run — so the recipe must not
# die on the very finding it exists to fix (masked below). The scan is also
# narrowed to Rust: ejectest moves inline RUST test modules, and a near-limit
# Markdown file has nothing to move.
eject PCT='90':
    #!/usr/bin/env bash
    set -eo pipefail

    # Mask linecop's finding-exit. `--baseline` is advisory by design — the
    # hard over-limit gate is `just check-file-size`, which runs bare linecop —
    # so exit 1 here means "found files to act on", and must not abort the
    # pipeline before ejectest sees them.
    paths="$(linecop --baseline {{ PCT }} --format paths || true)"

    # Rust only: paths outside `src` and non-`.rs` files are dropped by
    # ejectest's own --lenient root check anyway, but a Markdown path must not
    # reach a tool that moves inline Rust test modules at all.
    rust_paths="$(printf '%s\n' "$paths" | grep '\.rs$' || true)"

    if [ -n "$rust_paths" ]; then
        printf '%s\n' "$rust_paths" | ejectest apply src --files-from - --lenient
    fi

# Check for oversized files (fails if any exceed limits)
check-file-size:
    linecop

# Copy-paste gate: fail on any duplicated Rust block across the crate.
check-dry:
    # jscpd (the Rust port, packaged via qahq in flake.nix) fails on any
    # duplicated block of >=70 tokens, naming both spans. Fix a finding by
    # extracting a shared helper — never by shuffling tokens until the
    # detector loses the scent.
    #
    # `--exit-code 1` is spelled with its value: bare `--exit-code` would
    # swallow the following path argument as its optional code.
    #
    # `-r ai` because the default console reporter draws a 10-line table to
    # say `0 clones`; the ai reporter says it in 4 and still prints one
    # `a.rs:1-14 ~ b.rs:1-14` line per clone when it fails.
    jscpd --no-tips -k 70 -f rust --exit-code 1 -r ai src tests

# Dependency audit: advisories, license policy, and dependency bans. Kept out
# of `just check` on purpose — it fetches the advisory database, so it needs
# network and has no business blocking an offline commit. CI runs it in its own
# job, and `just validate` (in the template repo) runs it against a freshly
# generated project, so deny.toml can't silently rot out of schema.
#
# `--hide-inclusion-graph` because the default prints the FULL inclusion tree
# for every crate it has anything to say about, and a `bans` policy warns once
# per duplicate version in the lock. Measured on a real project: **1411 lines
# to say `advisories ok, bans ok, licenses ok`**, against 145 with the flag and
# the identical verdict. The graph is what you want when you are chasing one
# finding, not what you want on every green run — re-run without the flag then,
# which is one word and a fresh answer.
deny:
    cargo deny check --hide-inclusion-graph advisories licenses bans

# List dependencies with newer versions available. Needs network, and is a dev
# aid rather than a gate: an upstream release is not a reason for CI to go red.
# Unrelated to `just outdatty-check`, which is about doc drift inside this repo,
# not dependency versions.
outdated:
    cargo outdated --workspace

# Point git at the tracked pre-commit hook (`.githooks/pre-commit` asserts
# `mmz --is-fresh --tag gate` against the worktree, running nothing of its
# own). The dev shell runs this recipe from shellHook on entry, so a fresh
# clone's first `nix develop` wires the hook and edits to the tracked file
# take effect live (no copy to refresh). Bypass a single commit with
# `git commit --no-verify`.
install-hooks:
    git config core.hooksPath .githooks
    rm -f .git/hooks/pre-commit
    chmod +x .githooks/pre-commit
    echo "installed .githooks"

# The tag push is the only publish path: never `cargo publish` by hand, since a
# version already on crates.io used to fail the tag's workflow and leave the
# release half-done. One command does the whole release, so the push of main
# can't be forgotten and no tag lands on a red commit: every check that needs
# no push runs first; then main is pushed (fast-forward only), the recipe waits
# for the `ci.yml` run on that exact commit, and tags only if it is green. Each
# refusal prints `error:` plus a `hint:` with the way out. `--dry-run` runs the
# checks and says what it would push. A tag whose workflow failed is re-run on
# the same tag, never re-tagged: `gh workflow run <release workflow> -f tag=vX.Y.Z`.
# Push main, wait for its CI, then tag (usage: just release 0.1.0 [--dry-run])
[no-exit-message]
release VERSION *FLAGS:
    #!/usr/bin/env bash
    set -euo pipefail
    version='{{ VERSION }}'
    tag="v$version"
    dry_run=false
    refuse() { echo "error: $1" >&2; shift; for h in "$@"; do echo "hint: $h" >&2; done; exit 1; }
    for flag in {{ FLAGS }}; do
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
    workflow=$(basename "$(ls .github/workflows/release*.yml | head -1)")
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

# Pre-PR mutation check — the daily driver. Sweeps only mutants touched by
# this branch's diff against main, so it needs no state file and covers new
# files for free (every line of a new file is "touched"). A surviving MISSED
# mutant is a real assertion gap: a line covered but not pinned.
#
# Parallelism defaults to 3 and is overridden from the environment
# (`CARGO_MUTANTS_JOBS=8 just mutants-diff`). The default is resolved by just's
# `env()` rather than a shell `export`, so it is attached to the one command
# that reads it instead of living in the recipe's whole shell.
mutants-diff *ARGS:
    #!/usr/bin/env bash
    set -eo pipefail
    diff_file=$(mktemp)
    trap 'rm -f "$diff_file"' EXIT
    git diff "$(git merge-base main HEAD)" > "$diff_file"
    CARGO_MUTANTS_JOBS='{{ env("CARGO_MUTANTS_JOBS", "3") }}' \
        cargo mutants --in-diff "$diff_file" {{ ARGS }}

# Full-file mutation sweep — a dev aid, never a gate. Extra args pass through
# to cargo-mutants. File set and excludes come from .cargo/mutants.toml.
mutants *ARGS:
    cargo mutants {{ ARGS }}
