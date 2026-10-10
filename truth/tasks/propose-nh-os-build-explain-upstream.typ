#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Propose nh os build --explain upstream",
  priority: ice(
    impact: 6,
    confidence: 0.5,
    ease: 4,
  ),
  difficulty: 6,
  kind: "milestone",
  tags: ("upstream",),
  estimate: (src-files: 0),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[nh calls the library instead of a second binary],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "max",
  status: proposed(2026, 10, 10),
  links: depends-on("explain-why-each-root-misses-with-evidence.typ"),
)

== Problem

+ Run `nh os build --help` from `~/projects/clone/nh`.
+ 🔴 There is no `--explain`, and no plugin mechanism to add one: `NHCommand` is a closed enum in `crates/nh/src/interface.rs`.


== TO-BE

- 🟢 An upstream issue or PR proposes `nh os build --explain`, calling `nix_why_build`'s library API on the toplevel `nh` already evaluates.


== Changes

- No file in this repo: the proposal lives upstream, written against `crates/nh/src/interface.rs` and the `os build` path.


== Out of scope

- Implementing it here. The maintainer owns the upstream conversation.


== Dogfood

Not applicable: it ships in nh, not here.

