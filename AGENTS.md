# nix-why-build

## Agent Rules

- See and use [Justfile](Justfile). Add any repeatable and regular operations there.
  `set shell` already gives every recipe line `bash -eo pipefail`, so a new recipe
  needs no `#!/usr/bin/env bash` + `set -eo pipefail` preamble. just runs each line
  in a fresh shell though — add a shebang only when the body needs one shell across
  lines (a variable read back later, a `trap`).
- At the end ensure that `just fix-check` is green.
- Tests: inline `#[cfg(test)]` units; CLI/integration in `tests/` (`assert_cmd` + `predicates`). `just fix-check` auto-ejects inline tests from oversized files via `ejectest`.
  `src/nix_tests.rs` runs the real `nix` against `tests/fixtures/` in a scratch
  store that nextest's setup script provides: run it with `just test`, not
  `cargo test`.
- `just check` holds the coverage floor (95% of lines) and the CRAP gate as one
  arm, `just coverage`; `just cover` then `just crap` run them by hand.
  `just cover` uses `cargo-llvm-cov` (not tarpaulin) — it merges child-process
  profraw, so subprocess-based e2e tests attribute correctly. If `just crap`
  flags a function, add tests or reduce its branching — don't raise the
  threshold to dodge it.
- `just check` also runs rustdoc with `-D warnings`, `nixfmt --check` over the
  tracked `.nix` files and `actionlint` over the workflows; `just fmt` formats
  Rust and Nix.
- CI's check job also runs `nix flake check`, which builds the package; run it
  after touching `flake.nix` or `Cargo.lock` (it takes a few minutes).
- Every arm of `just check` is `just mmz <subgate>`:
  [mmz](https://github.com/mlavrinenko/mmz) skips arms whose declared inputs are
  unchanged since they last passed, so a new or retargeted gate needs its rule in
  [.mmz/config.yaml](.mmz/config.yaml) — an unmatched command is a hard error, not
  a silent skip. Don't wrap `just check` arms in `chronic`: a cache hit is already
  quiet, and a miss must stream its output.
- Be careful with the context. Omit non-necessary command outputs using `chronic` or `grep`.
- [outdatty.yaml](outdatty.yaml) couples sources to dependents. When `just check`
  reports drift, update the listed dependents, run `just outdatty-update`, and
  commit the refreshed `outdatty.lock` with the change. Add a group whenever you
  introduce files that must stay in sync.
- `just check-dry` (jscpd) flags duplicated Rust blocks of >=70 tokens. Fix
  a finding by extracting a shared helper — never by shuffling tokens until
  the detector loses the scent.
- `just deny` (cargo-deny: advisories, licenses, bans) is not in `just check` —
  it needs network. Run it after touching dependencies. Widen `deny.toml`'s
  allow-list deliberately; never to silence a finding. `just outdated`
  (cargo-outdated) is network-bound too and reports newer upstream releases;
  it is a dev aid, never a gate.
- `just test` gates the suite on cargo-nextest with `--status-level fail` — a
  green run prints only the summary, a red one prints each failure's full
  output — plus a `cargo test --doc` pass, because nextest does not run
  doctests. `just cover` instruments that same runner via `cargo llvm-cov
  nextest`, so a test cannot pass under the gate and fail under coverage (or
  the reverse) with nothing saying so.
- The Linux release binaries are `nix build .#static` (musl, so they run on
  NixOS). `nix flake check` builds only `.#default`, so after adding a
  dependency with C code, build `.#static` and run `file result/bin/*`: it
  must say statically linked.
- Release only with `just release X.Y.Z`, after `just release X.Y.Z --dry-run`
  passes; never `cargo publish` by hand, and don't push main first. The recipe
  refuses a dirty tree, an origin/main that main lacks commits of, an existing
  tag, a version already on crates.io, a CHANGELOG without a dated `X.Y.Z`
  section, stale gates and a failing `cargo publish --dry-run`. Then it pushes
  main, waits for `ci.yml` on that commit, and tags only if it is green; a red
  run leaves main pushed and no tag. A release whose workflow failed
  is finished on the same tag with `gh workflow run <release workflow> -f
  tag=vX.Y.Z`; a pushed tag never moves.
- Parallel agents each get a worktree from `just fork <name>`: a branch,
  `../nix-why-build-<name>` and its dev shell allowed. Gates tolerate several
  forks at once; merge each back and re-run `just check` on the result.
- Eating your own dog food: the tool should use itself if applicable.
- `just oracle <installable>` diffs `to_build` with nix's own dry-run plan on an
  empty store; it must print nothing, and it refuses an empty `to_build`. CI
  runs it on our own package (`.#default`) in its own job. After touching the
  walk (`src/walk.rs`, `src/drv.rs`), also run it on a real configuration.
- Tasks live in `truth/tasks/` under MindTape: load the `mindtape` skill, use
  `mt` for every task change, and flip a task with the commit that carries its
  work (`Refs: <stem>` footer). See Tasks in CONTRIBUTING.md.

See [CONTRIBUTING.md](CONTRIBUTING.md) for project conventions and code standards.
