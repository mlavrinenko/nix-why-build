#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Explain why each root misses, with evidence",
  priority: ice(
    impact: 8,
    confidence: 0.5,
    ease: 3,
  ),
  difficulty: 8,
  kind: "milestone",
  tags: ("eval", "report"),
  estimate: (src-files: 0),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[each root carries a category and the attributes that differ],
      fast: hurts[evaluating nixpkgs' counterparts costs seconds],
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
  links: depends-on("name-the-option-and-module-file-requesting-each-root.typ"),
)

== Problem

+ Run the milestone-3 build against tank.
+ 🔴 `llama-cpp-0.5.0` is a root requested by `apps/llama-cpp-16gb.nix`, but nothing says it misses because of its `postPatch`.


== TO-BE

- 🟢 Each root carries one category: overridden (with the differing env, structured attrs and inputs against nixpkgs' drv), unfree, not built by Hydra, nixpkgs newer than the cache, custom package, uncached source, or unknown.
- 🟢 The comparison uses the attribute the cache builds, so a `cudaSupport` override diffs against `pkgsCuda`.
- 🟢 No category is printed without its evidence.


== Changes

- `src/reason.rs`: the classifier.
- `src/diff.rs`: the drv-against-drv diff.
- `src/eval.rs`: evaluate the counterpart candidates and their meta.
- `src/report.rs`: a `why` line per root.


== Out of scope

- Querying Hydra's API for a failed build log. Filed once the categories prove too coarse.


== Dogfood

The tank report says `llama-cpp-0.5.0` is overridden and names `postPatch`.

