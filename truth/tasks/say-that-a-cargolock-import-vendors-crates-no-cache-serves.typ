#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Say that a cargoLock import vendors crates no cache serves",
  kind: "dev",
  priority: ice(
    impact: 6,
    confidence: 0.9,
    ease: 5,
  ),
  difficulty: 5,
  estimate: (src-files: 4),
  tags: ("found", "report"),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: helps[224 roots of one package fold into one line with one fix],
      names-requester: neutral,
      names-reason: helps[134 `unknown` roots get a category and the fix],
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 11),
)

== Problem

+ Writing the README example from a run on the maintainer's desktop NixOS config.
+ 🔴 #raw("src/reason.rs"): A package built with \`cargoLock.lockFile\` unpacks each crate in its own derivation, which no cache serves, so its crates print as \`unknown\` and \`fetch\` roots that never name the fix.

```
224 roots: arraydeque-0.5.1, arrayvec-0.7.8, assert_cmd-2.2.2 (+221 more)
  why unknown (134), fetch (90)
  drags in kache-1.0.0
```
Each `unknown` has no pname, and its `buildCommand` only untars `crate-arraydeque-0.5.1.tar.gz`. Each `fetch` gets `static.crates.io`. Chains run `<crate>`, `cargo-vendor-dir`, `kache-1.0.0`.


This fell outside the task that surfaced it, so it is filed here rather than
left in a closing report no one rereads. It belongs in
#raw("src/reason.rs").

== TO-BE

- 🟢 The crates of a `cargoLock.lockFile` package share one category, like `vendored: cargoLock.lockFile unpacks each crate locally; cargoHash vendors them in one fetch`.
- 🟢 The group still drags in the package, so the reader sees which `package.nix` to change.


== Changes

- `src/reason_tests.rs`: the red test, an unpack-a-crate root classified as vendored.
- `src/reason.rs`: a `Vendored` category, found before the pname lookup: a root whose only dependents are a `cargo-vendor-dir`, or whose `buildCommand` untars a `crate-*.tar.gz`. The crate fetches it drags along join it.
- `src/lookup.rs`: skip the nixpkgs lookup for vendored roots.
- `src/style.rs`, `src/human.rs`: the category's color, quiet like `fetch`.
- `docs/report.schema.json` (`just schema`), `README.md` (the reasons table), `CHANGELOG.md`.


== Rollback

- Revert the commit. The crates go back to `unknown` and `fetch`.


== Out of scope

- Go, npm and other per-dependency vendoring. Same idea, different markers; one at a time once someone hits it.


== Dogfood

The maintainer's desktop run prints one `vendored` group for `kache` instead of 224 roots split between `unknown` and `fetch`.

