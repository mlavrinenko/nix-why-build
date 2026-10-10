#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Diff a derivation against its nixpkgs counterpart",
  kind: "feat",
  priority: ice(
    impact: 8,
    confidence: 0.8,
    ease: 6,
  ),
  difficulty: 4,
  tags: ("walk",),
  estimate: (src-files: 2),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[an overridden root carries the attributes and inputs that differ],
      fast: neutral,
      embeddable: helps[the diff is data a caller can render its own way],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
  links: parent("explain-why-each-root-misses-with-evidence.typ"),
)

== Problem

+ Compare `/nix/store/qc9h4fl86p1yinqnkl361qs38gclk6vj-llama-cpp-0.5.0.drv` with `pkgsCuda.llama-cpp` from tank's `unstable` input by hand: `nix derivation show` both, strip hashes, `diff`.
+ 🔴 Nothing in the tool does this, so "overridden" would be a claim without its evidence.


== TO-BE

- 🟢 `diff::Diff::between(counterpart, root)` names the attributes added, removed and changed (env and structured attrs alike, store hashes ignored), the inputs added and removed by name, and the inputs whose name matches but whose `.drv` differs.
- 🟢 For tank's llama-cpp against `pkgsCuda.llama-cpp` it reads: `postPatch` added, `cmakeFlags` and `buildInputs` changed, input `blas-3` added.


== Changes

- `src/diff_tests.rs`: red tests first, over the fake store.
- `src/diff.rs`: normalized attributes and the diff.
- `src/drv.rs`: read a derivation's attributes and inputs in full.


== Alternatives

- Diff values line by line: a 40-line `postPatch` would drown the one fact that it differs.


== Out of scope

- Finding the counterpart. The next leaf evaluates it.


== Dogfood

Not applicable alone; the classifier leaf prints it for tank's llama-cpp.

