# Contributing to nix-why-build

## Development

You need [Nix](https://nixos.org/) with flakes. Enter the dev shell with
`direnv allow` or `nix develop`; it also wires the git hooks. Then:

```bash
just          # the recipes you need daily
just --list   # all of them, grouped
```

Before you send a change, run `just fix-check`: it applies clippy's fixes and
the formatters, then runs `just check`.

## Code Style

All clippy lints are set to `deny`, so a violation does not compile. Key
restrictions:

- No `unwrap()`; use `?` with `anyhow` or `thiserror`
- No `todo!()`, `unimplemented!()`, `unreachable!()`; handle every case
- No `unsafe`, no wildcard imports (`use foo::*`)
- Names of at least two characters
- Functions: at most 70 lines, 5 arguments, cognitive complexity 20

## Error Handling

- `anyhow::Result` in the binary, `thiserror::Error` for library errors callers
  match on.
- Propagate with `?`, never `unwrap()` or `expect()`.

## Project Structure

`main.rs` stays a thin entry point: argument parsing, logger init and a call
into the library. Logic belongs in `lib.rs` and its modules. `main.rs` is
excluded from coverage, so anything there goes untested.

## Testing

- Unit tests live inline in a `#[cfg(test)] mod tests` next to their code.
- CLI and integration tests live in `tests/` and drive the binary with
  [`assert_cmd`](https://docs.rs/assert_cmd) and
  [`predicates`](https://docs.rs/predicates), see `tests/cli.rs`.
- `just test` runs `cargo nextest run`, one process per test, plus
  `cargo test --doc`, which nextest skips. A green run prints the summary line.
  `just cover` instruments the same runner. The output level lives in
  [.config/nextest.toml](.config/nextest.toml), because `cargo llvm-cov
  nextest` ignores `--status-level` on the command line.
- `src/nix_tests.rs` evaluates `src/counterparts.nix` and `src/index.nix` with
  the real `nix` on `tests/fixtures/`, offline. They write `.drv` files, so a
  nextest setup script (`tests/support/nix-scratch-store.sh`) hands them an
  in-memory store through `NIX_REMOTE`. They fail under plain `cargo test`.
- Near the linecop limit, `just fix-check` ejects an inline `#[cfg(test)]`
  module into a sibling `_tests.rs` with
  [ejectest](https://github.com/mlavrinenko/ejectest).

## Coverage and CRAP

`just cover` enforces 95% line coverage with `cargo-llvm-cov`, which counts
subprocess e2e tests too. It starts with `cargo llvm-cov clean --workspace`,
or a long-lived checkout keeps scoring binaries that no longer exist.

`just crap` scores each function by the Change Risk Anti-Patterns metric
(complexity weighted by coverage) and fails above 30. A global floor can stay
green while one branchy, untested function rots; CRAP catches that. Fix a
flagged function with tests or less branching, never a higher threshold. It
refuses a stale `target/coverage/lcov.info`, since a function with no coverage
record grades as 0%. `tests/**` and `*_tests.rs` are excluded.

`just check` runs both, in order, as `just coverage`. Both are silent on a pass.

## File Size Limits

Rust files stop at 500 lines, Markdown at 200. Split a file that grows past
its limit into modules or separate documents.

## Dependency Drift

[outdatty.yaml](outdatty.yaml) couples `source` files to the `dependents` that
must follow them, like the CLI code and the README. `just check` fails when a
source changed and its dependents were not re-confirmed. Review them, update
what needs it, run `just outdatty-update` and commit `outdatty.lock` with the
change. Add a group for any new files that must move together.

## Memoized Gates

Every arm of `just check` runs as `just mmz <arm>`, which is `mmz just <arm>`.
[mmz](https://github.com/mlavrinenko/mmz) skips an arm while its declared
inputs match the last run that passed, so a no-op re-run costs nothing and a
one-file change re-runs only the arms that read it. Rules and their inputs
live in [.mmz/config.yaml](.mmz/config.yaml).

- A new arm needs a rule there. mmz rejects a command that matches no rule.
- Rules match by prefix on whole argv tokens. `just test` is `match: exact`,
  so a filtered `just test <filter>` never records the full-suite run.
- `mmz --is-fresh --tag gate` asserts that every `gate`-tagged rule passed
  against this worktree, running nothing. A tag that matches no rule passes
  vacuously, so a new arm must carry it. `just cover` and `just crap` are
  untagged; `just coverage` carries them.
- There is no `--force`. Touch an input or delete the record under
  `.mmz/cache/` to re-run a fresh rule.
- `just status` prints each rule's freshness.
- A brand-new clone has no `Cargo.lock` until the first `just check` writes
  it, so the second run misses once more. That's not a broken cache.

## Rustdoc, Nix and Workflows

`just doc` builds rustdoc with `-D warnings`, so a broken intra-doc link fails
before docs.rs renders it. `just nix-fmt` checks every tracked `.nix` file and
`just fmt` rewrites them. `just actionlint` lints `.github/workflows`,
including the shell in each `run:` step.

CI's check job also runs `nix flake check`, which builds the package
`nix run` installs, so a `Cargo.lock` the Nix vendoring rejects fails there.

## Copy-Paste Gate

`just check-dry` runs [jscpd](https://github.com/kucherenko/jscpd) over `src/`
and `tests/` and fails on any duplicated Rust block of 70+ tokens. Fix it by
extracting a shared helper, never by shuffling tokens until the detector
loses the scent.

## Dependency Audits

`just deny` runs `cargo-deny` (advisories, licenses, bans). It needs network,
so it runs in its own CI job, not in `just check`. The policy is
[deny.toml](deny.toml): add a license to `licenses.allow` deliberately, never
to silence a finding. `just outdated` lists newer dependency versions, also
network-bound and never a gate.

## MSRV

`rust-version` in `Cargo.toml` is the minimum supported Rust. A CI job builds
on it.

## Mutation Testing

`cargo-mutants` injects bugs and checks that the tests catch them.
`just mutants-diff` sweeps what this branch changed against `main`, the daily
driver. `just mutants` sweeps every file, slow and never a gate. A surviving
`MISSED` mutant is a line covered but pinned by no assertion.

## Git Hooks

Both hooks live in `.githooks/`. The dev shell runs `just install-hooks` on
entry, which points `core.hooksPath` there.

- `pre-commit` runs `mmz --is-fresh --tag gate`, so it runs no gate itself.
- `commit-msg` runs `mt check --commit-msg` (see Tasks).

Skip them for one commit with `git commit --no-verify`.

## Tasks

The backlog lives in `truth/tasks/`, one Typst file per task, tracked with
[MindTape](https://github.com/mlavrinenko/mindtape) (`mt`). `truth/goals.typ`
says what the tool is for; every open task gives each goal a verdict.

- File with `mt add -t feat` (or `fix`, `dev`, `docs`, `finding`, `tweak`).
  Start with `mt wip <stem>`, close with `mt done <stem>`, each flip staged
  with the work, and name the task in a `Refs: <stem>` footer.
- `mt done` refuses until `just check` passed. Task files sit outside the
  gate's scope, so one `mt done leaf milestone` closes both; the commit-msg
  hook validates the staged tasks.
