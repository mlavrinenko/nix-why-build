#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Group misses by root, with cascade chains",
  priority: ice(
    impact: 9,
    confidence: 0.8,
    ease: 6,
  ),
  difficulty: 5,
  kind: "milestone",
  tags: ("report",),
  estimate: (src-files: 0),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: helps[the report is a handful of roots, each with what it drags in],
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
  links: depends-on(
    "list-uncached-packages-for-a-flake-configuration-store-independent.typ",
  ),
)

== Problem

+ Run the milestone-1 build of `nix-why-build .#nixosConfigurations.tank`.
+ 🔴 It prints a flat list, so `coreutils-9.11` reads as a problem of its own, not as the cascade of an uncached `coreutils-9.11.tar.xz`.


== TO-BE

- 🟢 A root is an uncached derivation whose inputs are all served; every other miss is a cascade.
- 🟢 The report groups by root, each root listing the packages it drags in.
- 🟢 `--json` carries each cascade's chain down to its roots.


== Changes

- `src/roots.rs`: split the uncached graph into roots and cascades, with a shortest chain per cascade and root.
- `src/report.rs`: the grouped human output and the JSON shape.


== Out of scope

- Who requests a root, and why it misses. Milestones 3 and 4.


== Dogfood

The tank report shrinks to its roots; `llama-cpp-0.5.0` stands as a root.

