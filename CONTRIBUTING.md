# Contributing to nix-why-build

## Code Style

All clippy lints are set to `deny` level — the project will not compile with violations.

Key restrictions:
- No `unwrap()` — use `?` operator or `anyhow`/`thiserror` error handling
- No `todo!()`, `unimplemented!()`, `unreachable!()` — handle all cases
- No `unsafe` code
- No wildcard imports (`use foo::*`)
- No single-character variable names (minimum 2 characters)
- Functions: max 70 lines, max 5 arguments, max cognitive complexity 20

## Error Handling

- Use `anyhow::Result` for application-level code (binaries, CLI)
- Use `thiserror::Error` for library error types that callers will match on
- Propagate errors with `?` — never `unwrap()` or `expect()`

## Project Structure

Keep `main.rs` as a thin entry point — argument parsing, logger init, and a call into
library code. All logic belongs in `lib.rs` (and its modules). `main.rs` is excluded from
coverage, so anything there is untested by default.

## Testing

- Unit tests live inline in a `#[cfg(test)] mod tests` block next to the code they exercise.
- CLI and integration tests live in `tests/` and drive the built binary with
  [`assert_cmd`](https://docs.rs/assert_cmd) + [`predicates`](https://docs.rs/predicates)
  (see `tests/cli.rs`).
- Run the full suite with `just test`: `cargo nextest run --status-level fail`
  (one process per test, so a panic or hang is isolated; a green run prints only
  the summary, a failure prints the full output) plus `cargo test --doc` for
  doctests, which [cargo-nextest](https://nexte.st) does not run. `just cover`
  instruments the same nextest runner, so the gate and coverage can't disagree
  about whether a test passes. The output level lives in
  [.config/nextest.toml](.config/nextest.toml), not in the recipe: `cargo
  llvm-cov nextest` accepts `--status-level fail` and ignores it, so a coverage
  run printed one `PASS` line per test until the profile moved there.
- As a file approaches the linecop limit, `just fix-check` ejects its inline
  `#[cfg(test)]` module into a sibling `_tests.rs` file via
  [ejectest](https://github.com/mlavrinenko/ejectest), driven by `linecop --baseline`.
  This keeps source files under the limit without losing the inline-test workflow.

## Code Coverage

Minimum 70% line coverage enforced via `cargo-llvm-cov`. Run `just cover` to check.
llvm-cov merges the profraw of child processes, so subprocess-based e2e tests
attribute correctly — unlike tarpaulin, which misses them. It starts with
`cargo llvm-cov clean --workspace`, and that line is not tidiness: `--no-report`
deliberately keeps the profraw pile so runs can be merged, so without the clean
a long-lived checkout scores binaries that no longer exist. Never trade it for
speed — it fails toward pessimism, which no floor complains about.
`main.rs` is excluded from coverage — keep it thin and move testable logic to `lib.rs`.

## CRAP Gate

`just crap` scores each function by the Change Risk Anti-Patterns metric
(cyclomatic complexity weighted by test coverage) and fails above 30. A global
coverage threshold can stay green while one branchy, untested function rots;
CRAP catches that. Fix a flagged function by adding tests or reducing its
branching. Tune the threshold per repo via `--threshold` or a `.cargo-crap.toml`.

It reads `target/coverage/lcov.info` and **refuses a stale one** rather than
scoring it, because cargo-crap's `--missing pessimistic` grades a function with
no coverage record as 0% — so every file added since the last `just cover` reads
as completely untested and arrives as a plausible finding. `just
_lcov-stale-reason` is that test; run `just cover` first (CI and `just validate`
chain them). The same rule is why `tests/**` and every ejected `*_tests.rs` are
excluded: llvm-cov instruments the code under test, not the suite, so without
those excludes the gate grades your test helpers. `--min 30`, matched to the
threshold, is what keeps a green run quiet — the table then prints only what
fails, and a red run still names every offender.

## File Size Limits

- Rust files: 500 lines max
- Markdown files: 200 lines max

When a file exceeds the limit, split it into modules or separate documents.

## Dependency Drift

[outdatty.yaml](outdatty.yaml) declares groups that couple `source` files to the
`dependents` that must stay in sync with them — for example, CLI code to the docs
that describe it. `just check` runs `outdatty check`, which fails when a source
changed but its dependents were not re-confirmed.

After editing a source, review the listed dependents, update them as needed, then
run `just outdatty-update` to record the new state into `outdatty.lock` and commit
it. Add or adjust groups whenever you introduce files that must move together.

## Memoized Gates

Every arm of `just check` runs as `just mmz <subgate>`, which is
`mmz just <subgate>`. [mmz](https://github.com/mlavrinenko/mmz) hashes the rule's
declared inputs and skips the command when they are byte-for-byte identical to
the last run that *succeeded*, so a no-op `just check` re-run costs nothing and a
one-file change re-runs only the arms that declare that file.

Rules and their input scopes live in [.mmz/config.yaml](.mmz/config.yaml). Things
worth knowing:

- Adding an arm to `just check` means adding its rule to that manifest. mmz's
  default `strict` setting rejects an invocation matching no rule (and one whose
  rule resolves to zero files), so a missing rule is an error, not a silent pass.
- Rule names match by prefix on whole argv tokens. `just test` is `match: exact`
  so a filtered `just test <filter>` can never record the full-suite identity.
- Rules tagged `gate` are the gate set: `mmz --is-fresh --tag gate` asserts every
  one already passed against this worktree and runs nothing. That is what the
  pre-commit hook checks (see Git Hooks below), and what to wire into a task
  tracker's close gate. `just cover` and `just crap` are untagged — memoized,
  but never blocking. A tag matching no rule asserts nothing and passes
  vacuously, so any new gate arm must carry the `gate` tag to be covered.
- There is no `--force`. To re-run a fresh rule, touch one of its inputs or
  delete its record under `.mmz/cache/` (gitignored via `.mmz/.gitignore`).
- `just status` (`mmz --status`) prints every rule's freshness as a table — what
  a `just check` would re-run, and whether the pre-commit hook would pass now.
- In a brand-new clone there is no `Cargo.lock` yet; cargo writes it during the
  first `just check`, so the arms that declare it record a pre-lock digest and
  miss once more on the second run. "A no-op re-run is a full hit" holds from
  the third run onward. Nothing to fix — just do not read that first miss as a
  broken cache.

## Copy-Paste Gate

`just check-dry` runs [jscpd](https://github.com/kucherenko/jscpd) over `src/`
and `tests/` and fails on any duplicated Rust block of >=70 tokens. Fix a
finding by extracting a shared helper — never by shuffling tokens until the
detector loses the scent.

## Dependency Audits

`just deny` runs `cargo-deny` over advisories, the license allow-list, and
dependency bans. It is deliberately not part of `just check`: it fetches the
advisory database, so it needs network and would make an offline commit fail.
CI runs it in its own job. The recipe passes `--hide-inclusion-graph`: the
default prints a full inclusion tree per crate it mentions, so a green run can
be 1411 lines to say `advisories ok, bans ok, licenses ok`. Drop the flag when
you are chasing one finding and want the tree.

The policy lives in [deny.toml](deny.toml). A dependency under a license not in
`licenses.allow` fails the check — add one deliberately, not to silence a
finding. Keep the file in cargo-deny's current schema: a retired key (the old
`advisories.vulnerability`, `licenses.copyleft`, ...) is rejected outright and
fails the whole check rather than being ignored.

`just outdated` (cargo-outdated) lists dependencies with newer versions
published. Also network-bound, and deliberately not a gate — an upstream release
is not a reason for CI to go red. Unrelated to `just outdatty-check` above,
which is about doc drift rather than dependency versions.

## MSRV

The `rust-version` field in `Cargo.toml` declares the minimum supported Rust
version. CI verifies it compiles on that version in a separate job.

## Mutation Testing

`cargo-mutants` helps find assertion gaps by injecting bugs and checking that
tests catch them. Two recipes:

- `just mutants-diff` — sweeps only mutants touched by this branch's diff
  against `main` (the daily driver).
- `just mutants` — full-file sweep (a dev aid, never a gate).

A surviving `MISSED` mutant is a real assertion gap: a line covered but not
pinned by any assertion.

## Git Hooks

Both hooks live in `.githooks/`, tracked by git. `just install-hooks` runs
`git config core.hooksPath .githooks`; the dev-shell `shellHook` runs it on
entry, so a fresh clone's first `nix develop` wires them.

- `pre-commit` runs `mmz --is-fresh --tag gate`: every gate-tagged rule in
  `.mmz/config.yaml` already passed against this worktree. It runs nothing.
- `commit-msg` runs `mt check --commit-msg` (see Tasks).

Bypass a single commit with `git commit --no-verify`.

## Tasks

The backlog lives in `truth/tasks/`, one Typst file per task, tracked with
[MindTape](https://github.com/mlavrinenko/mindtape) (`mt`). `truth/goals.typ`
says what the tool is for; every open task gives each goal a verdict.

- File with `mt add -t feat` (or `fix`, `dev`, `docs`, `finding`, `tweak`).
  Start with `mt wip <stem>`, close with `mt done <stem>`, each flip staged
  with the work, and name the task in a `Refs: <stem>` footer.
- `mt done` refuses until `just check` passed. The flip rewrites a task file,
  so run `just check` once more before committing.

## Submitting Changes

1. Run `just check`: fmt, clippy, tests, file size, drift, copy-paste, tasks
2. Run `just fmt` to format code
3. Ensure `just cover` meets the 70% threshold
