#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "List uncached packages for a flake configuration, store-independent",
  priority: ice(
    impact: 9,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 6,
  kind: "milestone",
  tags: ("walk", "cache"),
  estimate: (src-files: 0),
  x-goals: (
    product: (
      store-independent: helps[asks the substituters, never the local store],
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: helps[parallel narinfo requests replace a `nix path-info` per substituter],
      embeddable: helps[the walk is a library function the binary only calls],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: wip(2026, 10, 10),
)

== Problem

+ Run `nix build --dry-run .#nixosConfigurations.tank.config.system.build.toplevel` on a machine that already built `llama-cpp`.
+ 🔴 `llama-cpp` is missing from "will be built", though a fresh machine would compile it.
+ Run `~/.nix/scripts/to-build.sh tank` instead: it asks the caches, but takes 2m15s.


== TO-BE

- 🟢 `nix-why-build .#nixosConfigurations.tank` prints every pname-carrying derivation no configured substituter serves, the same list on any machine.
- 🟢 It reads substituters and trusted keys from `nix config show`, and `--substituter`/`--trusted-key` override them.
- 🟢 `--json` prints the same list for machines.


== Changes

- `src/config.rs`: substituters and trusted keys from `nix config show --json`.
- `src/eval.rs`: resolve the installable to its toplevel `.drv`.
- `src/drv.rs`: read `.drv` files with `nix-compat`.
- `src/cache.rs`: parallel narinfo GETs, a signature check against the trusted keys.
- `src/walk.rs`: the walk, stopping at served derivations.


== Out of scope

- Grouping misses by root. Milestone 2.
- Non-HTTP substituters (`ssh://`, `s3://`). They count as unknown, said once.


== Dogfood

Run it against `~/.nix` `.#nixosConfigurations.tank`; `llama-cpp-0.5.0` must show up.

