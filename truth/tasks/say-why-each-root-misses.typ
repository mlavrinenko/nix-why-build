#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Say why each root misses",
  kind: "feat",
  priority: ice(
    impact: 9,
    confidence: 0.6,
    ease: 4,
  ),
  difficulty: 6,
  tags: ("report",),
  estimate: (src-files: 4),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[each root carries a category and its evidence, or says unknown],
      fast: hurts[the counterpart evaluation runs after the walk],
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: helps[the README shows the categories on tank's output],
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
  links: parent("explain-why-each-root-misses-with-evidence.typ"),
)

== Problem

+ Run `nix-why-build .#nixosConfigurations.tank`.
+ 🔴 `llama-cpp-0.5.0` is a root wanted by `apps/llama-cpp-16gb.nix`, and nothing says it misses because its `postPatch` differs from what `cache.nixos-cuda.org` builds.


== TO-BE

- 🟢 Each root gets one category: `fetch` (an uncached fixed-output source, with its URL), `overridden` (with the diff against the closest cached counterpart of the same version), `unfree`, `not-built-by-hydra` (`meta.broken` or `meta.hydraPlatforms`), `newer-than-cache` (that input's `hello` is uncached too), `custom` (no attribute matched, with what was tried), or `unknown` with why.
- 🟢 The report prints a `why` line under each group; `--json` carries the reason with its evidence.


== Changes

- `src/reason_tests.rs`: red tests first, over the fake store.
- `src/reason.rs`: the categories and the choice of counterpart.
- `src/lib.rs`: run the counterpart evaluation after the walk, check the candidates' outputs against the caches.
- `src/report.rs`: the reason per root, the `why` line.


== Alternatives

- Ask Hydra's API why a job is missing: a network dependency on one cache's frontend, and it knows nothing of overrides.


== Out of scope

- Roots without a `pname` beyond fetches: they say unknown until someone needs better.


== Dogfood

The tank report says `llama-cpp-0.5.0` is overridden and names `postPatch`.

