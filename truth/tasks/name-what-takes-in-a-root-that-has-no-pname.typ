#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Name what takes in a root that has no pname",
  kind: "feat",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 7,
  ),
  difficulty: 3,
  tags: ("found", "report"),
  estimate: (src-files: 3),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[a pname-less root names the derivation that takes it in],
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 11),
)

== Problem

+ Run `nix-why-build --json --substituter https://cache.nixos.org/ ~/.nix#nixosConfigurations.mlavrinenko` on the maintainer's desktop.
+ 🔴 #raw("src/reason.rs"): 45 roots, like `config.toml`, `Cargo.lock` and `estoola-mcp-wrapper`, all say `unknown: it has no pname to look up in nixpkgs`, which tells a reader nothing about where each one comes from.


== TO-BE

- 🟢 A pname-less root names the derivations that take it in as an input, like `unknown: it has no pname to look up in nixpkgs; an input of home-manager-files`.
- 🟢 Several are named in name order, three at most, like `a, b, c +2`; a root nothing else takes in, the top itself, keeps today's text.


== Changes

- `src/reason_tests.rs`: the red test, a pname-less root naming its dependents.
- `src/reason.rs`: `classify` takes the names of the root's dependents and adds them to the no-pname reason.
- `src/lookup.rs`: collect each root's dependents from the graph, once per run.
- `CHANGELOG.md`.


== Alternatives

- Name the nearest dependent with a pname. The `drags in` line already names it, so it says nothing new.
- A structured `input_of` field in `--json`. Each root's `cascades` already carry the chains, so only the human line lacks it.
- Rewrite the reason in #raw("src/lookup.rs") after `classify`. String surgery on a finished reason; `classify` owns the wording.


== Out of scope

- A category of its own for config glue. That needs a marker for "your config", not a dependent's name.


== Dogfood

The desktop run's 45 pname-less roots each name what takes them in.

