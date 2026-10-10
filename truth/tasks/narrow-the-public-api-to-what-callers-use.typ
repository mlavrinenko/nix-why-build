#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Narrow the public API to what callers use",
  priority: ice(
    impact: 6,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 4,
  kind: "refactor",
  tags: ("api",),
  estimate: (src-files: 5),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[`nh` links against a small surface that can grow without breaking it],
    ),
    process: (
      sound-codebase: helps[internal modules can change shape without a semver break],
      docs-match-code: helps[docs.rs shows the handful of items a caller needs, not every module],
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
)

== Problem

+ Read `src/lib.rs`: all 14 modules are `pub mod`, so `walk::State`'s neighbours, `drv::read`, `cache::Substituters`, `lookup::classify_roots` and the rest are public API.
+ 🔴 Once 0.1.0 is on crates.io every one of them is a semver promise: renaming a walk helper is a breaking change. `Options` has public fields and `Reason`, `Report` and its item types are exhaustive, so adding a field or a category breaks every caller too.


== TO-BE

- 🟢 The crate root exports `explain`, `Options`, `Error`, `Report` and its item types (`Root`, `Package`, `Requested`, `Cascade`, `Unknown`), `Reason` and `Diff`; every module is private.
- 🟢 `Options`, `Report`, its item types and `Reason` are `#[non_exhaustive]`; `Options` is built with `Options::new` plus `with_substituters`, `with_trusted_keys` and `with_store_dir`.
- 🟢 The README and crate-doc library examples compile and run as doctests, unchanged in what they show.
- 🟢 Code left unused once private is deleted, or moved under `#[cfg(test)]` when only tests use it.


== Changes

- `src/lib.rs`: red step first, a doctest that builds `Options` through the builders; then private modules and re-exports.
- `src/main.rs`: build `Options` through the builders.
- `src/report.rs`, `src/reason.rs`: `#[non_exhaustive]`.
- `src/cache.rs`: drop or test-gate builders only tests call.


== Out of scope

- Renaming anything: the names are fine, only their reach changes.
- The `--json` schema: it is the CLI's contract, already stable.


== Dogfood

`cargo doc --open` in this repo lists the dozen items `nh` would use.

