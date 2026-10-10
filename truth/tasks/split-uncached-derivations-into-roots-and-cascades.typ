#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Split uncached derivations into roots and cascades",
  kind: "feat",
  priority: ice(
    impact: 9,
    confidence: 0.8,
    ease: 6,
  ),
  difficulty: 4,
  tags: ("walk",),
  estimate: (src-files: 2),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: helps[every build names the roots it cascades from],
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[the split is library data, not printed text],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
  links: parent("group-misses-by-root-with-cascade-chains.typ"),
)

== Problem

+ Run `nix-why-build .#nixosConfigurations.tank`.
+ 🔴 It prints 45 packages flat; nothing says which of them build only because something below them does.


== TO-BE

- 🟢 A root is a derivation that gets built while every input it uses is served; every other build is a cascade.
- 🟢 Each cascade knows its roots and one shortest chain down to each.


== Changes

- `src/roots_tests.rs`: red tests first, over the fake store.
- `src/roots.rs`: the split, memoized over the build subgraph, with a breadth-first chain per cascade.
- `src/walk.rs`: expose each build's build inputs.


== Alternatives

- All chains instead of a shortest one: exponential on a diamond-heavy graph, and one chain explains the cascade.


== Out of scope

- Printing. The next leaf.


== Dogfood

Not applicable alone; the report leaf runs it against tank.

