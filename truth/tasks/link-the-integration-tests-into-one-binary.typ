#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Link the integration tests into one binary",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 7,
  ),
  difficulty: 2,
  tags: ("tests", "tooling"),
  estimate: (src-files: 4),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: helps[the suite links every dependency once, not once per test file, and the shared fixture compiles once],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 11),
)

== Problem

+ Run `cargo test --no-run`, then `find target/debug/deps -maxdepth 1 -type f -executable | wc -l`.
+ 🔴 Cargo builds one executable per file in `tests/` (`cli.rs`, `color.rs`, `progress.rs`), and each links every dependency again, plus `tests/support/fixture.rs` and `chain.rs` compiled into each through `#[path = "support/..."]`. `just cover` builds the set a second time, instrumented.
+ Measured 2026-10-11 in mindtape: 74 test files gave `target/debug/deps` 263 executables averaging 298 MB, 80 GB in all; `just cover` took 51 GB in one worktree and the disk filled. canibump's 7 test files built 28 binaries of about 37 MB each with full debug info. Debug info size is handled host-wide in `~/.nix`; this task is the per-repo half.


== TO-BE

- 🟢 `tests/it/main.rs` declares `mod cli;`, `mod color;`, `mod progress;` and the shared `fixture` and `chain` modules once, so `cargo test --no-run` builds one integration binary.
- 🟢 The test count equals the baseline from before the move; nextest still runs each test in its own process, now named under `it::`.
- 🟢 Every gate, filter and doc that names `tests/cli.rs` or the old layout follows the move.


== Changes

- Baseline first: `just count-tests` and the executable count from `find target/debug/deps -maxdepth 1 -type f -executable | wc -l` after `cargo test --no-run`.
- `tests/cli.rs`, `tests/color.rs`, `tests/progress.rs`: `git mv` into `tests/it/`. Delete their `#[path = "support/..."]` plus `mod chain;` and `mod fixture;` lines; the `use chain::...` and `use fixture::...` lines become `use crate::chain::Chain;` and `use crate::fixture::{...};`.
- `tests/it/main.rs` (new): `mod cli; mod color; mod progress;` plus `#[path = "../support/chain.rs"] mod chain;` and `#[path = "../support/fixture.rs"] mod fixture;`. Keep `tests/support/` where it is: `src/lib.rs:49` includes `../tests/support/fixture.rs` as `fixture_tests`, and `.config/nextest.toml:25` runs `tests/support/nix-scratch-store.sh`, so neither path changes. `chain.rs` reaches `super::fixture`, which still resolves from the crate root. Their `#![allow(dead_code)]` stays.
- `Cargo.toml` has no `[[test]]` entries; add none unless autodiscovery fails.
- Prose that names the old files: `CONTRIBUTING.md:44` (`see tests/cli.rs` becomes `tests/it/cli.rs`) and `src/walk_tests.rs:245` (`shared with tests/cli.rs`). Already correct and left alone: `.mmz/config.yaml:33`, `.mindtape/config.toml:229`, `scripts/crap.sh:10,18` and `Justfile:130` all use `tests` or `tests/**`, which cover `tests/it/`; `AGENTS.md:11` and `CONTRIBUTING.md:42` say `tests/`. Re-run `grep -rn "tests/" --exclude-dir=target --exclude-dir=.git .` to confirm, and `just outdatty-update` only if a coupled path changed.
- Check: the test count equals the baseline, `cargo test --no-run` shows one integration executable, and `just fix-check` and `just check` are green.


== Rollback

- Move the files back to `tests/` and restore the three `#[path]` lines; nothing outside the test tree depends on the layout.


== Out of scope

- Debug info settings, which are host-wide in `~/.nix`.
- Inline `#[cfg(test)]` units, which already share one binary per crate.
- Moving `tests/support/` or `tests/fixtures/`: `src/lib.rs`, `src/nix_tests.rs` and the nextest setup script point at them.


== Dogfood

`just test` and `just cover` here link one integration binary instead of three, and `target/` stays smaller.

