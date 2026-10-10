#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Find the nixpkgs counterpart of each root",
  kind: "feat",
  priority: ice(
    impact: 8,
    confidence: 0.6,
    ease: 4,
  ),
  difficulty: 6,
  tags: ("eval",),
  estimate: (src-files: 4),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[a root is compared against what nixpkgs, or pkgsCuda, builds under the same attribute],
      fast: hurts[importing each nixpkgs input costs a few seconds],
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
  links: parent("explain-why-each-root-misses-with-evidence.typ"),
)

== Problem

+ Look for tank's `llama-cpp-0.5.0` in nixpkgs: it comes from `pkgs.unstable.llama-cpp.override { cudaSupport = true; ... }`, so the configuration's own `pkgs.llama-cpp` is another version from another input.
+ 🔴 Nothing finds the attribute a cache would build for it, so there is nothing to diff against.


== TO-BE

- 🟢 For each root with a `pname`, one `nix eval` imports every nixpkgs-like flake input of the configuration and looks the root up under a few attribute guesses (`<pname>`, its camelCase, `python3Packages.<pname>`, `perlPackages.<Pname>`), and under `pkgsCuda` when the root uses CUDA.
- 🟢 Each candidate carries input, attribute, `.drv`, version, `meta.unfree`, `meta.broken` and `meta.hydraPlatforms`; each input carries its `hello`, to tell whether the cache has caught up with that revision.


== Changes

- `src/counterparts_tests.rs`: red tests first, the expression run by real `nix eval` against a fake nixpkgs directory.
- `src/counterparts.nix`, `src/counterparts.rs`: the expression, the query and the candidates.
- `src/drv.rs`: keep the derivation's `system`.
- `src/modules.rs`: list the nixpkgs-like inputs of the flake archive.


== Alternatives

- The configuration's own `pkgs`: its overlays are part of what is being explained, and `pkgs.unstable`-style sets hide behind arbitrary names.
- An index of every pname in nixpkgs: minutes of evaluation for a handful of roots.


== Out of scope

- Deciding the category. The next leaf.


== Dogfood

Not applicable alone; the classifier leaf runs it against tank.

