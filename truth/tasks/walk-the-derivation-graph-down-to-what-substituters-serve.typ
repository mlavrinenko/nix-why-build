#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Walk the derivation graph down to what substituters serve",
  kind: "feat",
  priority: ice(
    impact: 9,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 5,
  tags: ("walk",),
  estimate: (src-files: 3),
  x-goals: (
    product: (
      store-independent: helps[the cut is the substituters' answer, never a local output],
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[the walk takes any query, so a caller can bring its own cache client],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
  links: parent(
    "list-uncached-packages-for-a-flake-configuration-store-independent.typ",
  ),
)

== Problem

+ Run `nix derivation show` over the closure, as `~/.nix/scripts/to-build.sh` does.
+ 🔴 The JSON shape moved between Nix versions (now `version: 4`, basenames, `structuredAttrs`), and the script parses it with jq.


== TO-BE

- 🟢 `walk::walk(store_dir, top, query)` reads `.drv` files with `nix-compat`, asks the query for each output a parent wants, and descends only into derivations that must be built.
- 🟢 A derivation that cannot be read or parsed (a floating content-addressed one) is unknown, never an empty path.
- 🟢 `pname` comes from `env` or from `__json` structured attrs.


== Changes

- `src/drv_tests.rs`, `src/walk_tests.rs`: red tests first, over a fake store dir of hand-written ATerm.
- `src/drv.rs`: read one `.drv` into name, pname, version, outputs and wanted inputs.
- `src/walk.rs`: the breadth-first walk, one batched query per level.
- `src/lib.rs`: export the modules.


== Alternatives

- `nix derivation show`: one subprocess per level and a JSON format that changes under us.
- Asking only `out`, as the prototype did: a parent wanting `dev` or `lib` would see the wrong cut.


== Out of scope

- Evaluating the installable. The CLI leaf.


== Dogfood

Not applicable alone; the CLI leaf runs it against tank.

