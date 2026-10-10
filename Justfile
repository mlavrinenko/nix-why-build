set quiet := true

# `-eo pipefail` on every recipe line. No `-u`: recipes read optional env
# and `{{ }}` values that may be empty. A body longer than ~5 lines lives in
# scripts/. Every recipe has a `[doc()]` and a group: `daily` is what bare
# `just` lists, `gate` the arms of `check`, `extra` the rest.
set shell := ["bash", "-eo", "pipefail", "-c"]
set ignore-comments := true

[private]
default:
    just --list --group daily

[doc("Apply fixes, then run check")]
[group("daily")]
fix-check: eject clippy-fix fmt check

# Each arm is memoized: mmz skips it while its inputs in .mmz/config.yaml
# are unchanged since it last passed.
[doc("Run every gate, skipping those whose inputs did not change")]
[group("daily")]
[parallel]
check: (mmz "fmt-check") (mmz "clippy") (mmz "test") (mmz "coverage") (mmz "doc") (mmz "nix-fmt") (mmz "actionlint") (mmz "machete") (mmz "check-file-size") (mmz "check-dry") (mmz "check-tasks") (mmz "outdatty-check")

# nextest skips doctests, hence the second pass. Green prints the summary.
[doc("Run the tests; ARGS filter them")]
[group("daily")]
[positional-arguments]
test *ARGS:
    out=$(cargo nextest run --workspace --status-level fail "$@" 2>&1) || { printf '%s\n' "$out"; exit 1; }; grep -m1 'Summary' <<< "$out"
    scripts/quiet.sh cargo test --workspace --doc "$@"

[doc("Build the binary")]
[group("daily")]
build *args:
    cargo build --workspace -q {{ args }}

[doc("Format Rust and Nix")]
[group("daily")]
fmt:
    cargo fmt --all
    nixfmt $(git ls-files '*.nix')

[doc("Show which gates check would re-run")]
[group("daily")]
status:
    mmz --status

[doc("Record that docs caught up with their sources")]
[group("daily")]
outdatty-update:
    outdatty update

[doc("Make ../nix-why-build-NAME on branch fork/NAME, dev shell allowed")]
[group("daily")]
fork NAME:
    git worktree add -q -b 'fork/{{ NAME }}' '../nix-why-build-{{ NAME }}' HEAD
    if command -v direnv >/dev/null; then direnv allow '../nix-why-build-{{ NAME }}'; fi
    echo "../nix-why-build-{{ NAME }} on fork/{{ NAME }}"

[doc("Install this checkout into ~/.cargo/bin")]
[group("daily")]
install:
    cargo install --path . --locked --quiet

# `mmz just X` makes `just X` the cache identity in .mmz/config.yaml.
[doc("Run a recipe through mmz")]
[group("gate")]
[no-exit-message]
mmz *ARGS:
    mmz just {{ ARGS }}

[doc("Check Rust formatting")]
[group("gate")]
fmt-check:
    cargo fmt --all -- --check

[doc("Lint with clippy")]
[group("gate")]
clippy:
    cargo clippy --workspace --all-targets -q -- -D warnings

[doc("Apply clippy's fixes")]
[group("gate")]
clippy-fix:
    cargo clippy --fix --workspace --all-targets --allow-dirty --allow-staged -- -D warnings

[doc("Build rustdoc, failing on any warning")]
[group("gate")]
doc:
    RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps -q

[doc("Hold the coverage floor and the CRAP gate")]
[group("gate")]
coverage: cover crap

[doc("Measure coverage, failing under 95% of lines")]
[group("gate")]
cover:
    scripts/cover.sh

[doc("Fail on complex, undertested functions; needs a fresh cover")]
[group("gate")]
crap:
    scripts/crap.sh

[doc("Check Nix formatting")]
[group("gate")]
nix-fmt:
    nixfmt --check $(git ls-files '*.nix')

[doc("Lint the GitHub workflows")]
[group("gate")]
actionlint:
    actionlint .github/workflows/*.yml

[doc("Find unused dependencies")]
[group("gate")]
machete:
    scripts/quiet.sh cargo machete

[doc("Fail on files over the linecop limits")]
[group("gate")]
check-file-size:
    linecop

[doc("Fail on duplicated Rust blocks of 70+ tokens")]
[group("gate")]
check-dry:
    scripts/quiet.sh jscpd --no-tips -k 70 -f rust --exit-code 1 -r ai src tests

[doc("Validate the task backlog")]
[group("gate")]
check-tasks:
    mt check -q

[doc("Fail when docs lag behind their sources")]
[group("gate")]
outdatty-check:
    outdatty check

[doc("Move inline tests out of Rust files near the size limit")]
[group("gate")]
eject PCT='90':
    scripts/eject.sh '{{ PCT }}'

[doc("Regenerate docs/report.schema.json from the report's types")]
[group("extra")]
schema:
    mkdir -p docs
    UPDATE_SCHEMA=1 just test the_documented_schema_matches_the_report

[doc("Diff to_build with nix's own dry-run plan")]
[group("extra")]
oracle INSTALLABLE: (build "--release")
    scripts/oracle.sh '{{ INSTALLABLE }}' '{{ invocation_directory() }}'

[doc("Push main, wait for its CI, then tag: just release 0.1.0 [--dry-run]")]
[group("extra")]
[no-exit-message]
release VERSION *FLAGS:
    scripts/release.sh '{{ VERSION }}' {{ FLAGS }}

[doc("Audit dependencies: advisories, licenses, bans (needs network)")]
[group("extra")]
deny:
    scripts/quiet.sh cargo deny check --hide-inclusion-graph advisories licenses bans

[doc("List newer dependency versions (needs network)")]
[group("extra")]
outdated:
    cargo outdated --workspace

[doc("Mutation-test what this branch changed")]
[group("extra")]
mutants-diff *ARGS:
    mkdir -p target
    git diff "$(git merge-base main HEAD)" > target/branch.diff
    CARGO_MUTANTS_JOBS='{{ env("CARGO_MUTANTS_JOBS", "3") }}' cargo mutants --in-diff target/branch.diff {{ ARGS }}

[doc("Mutation-test every file; slow")]
[group("extra")]
mutants *ARGS:
    cargo mutants {{ ARGS }}

[doc("Count the tests")]
[group("extra")]
count-tests:
    cargo nextest list --workspace --message-format json \
        | jq -r '[.["rust-suites"][].testcases[] | select(.ignored == false)] | "\(length) tests"'

[doc("Show the 20 longest files")]
[group("extra")]
file-sizes:
    find . -type f \( -name '*.rs' -o -name '*.md' \) ! -path './target/*' -exec wc -l {} + | sort -rn | awk 'NR <= 20'

[doc("Point git at .githooks; the dev shell does it on entry")]
[group("extra")]
install-hooks:
    git config core.hooksPath .githooks
    rm -f "$(git rev-parse --git-common-dir)/hooks/pre-commit"
    chmod +x .githooks/pre-commit
    echo "installed .githooks"
