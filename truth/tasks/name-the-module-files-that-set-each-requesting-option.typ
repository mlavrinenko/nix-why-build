#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Name the module files that set each requesting option",
  kind: "feat",
  priority: ice(
    impact: 8,
    confidence: 0.6,
    ease: 4,
  ),
  difficulty: 6,
  tags: ("eval", "report"),
  estimate: (src-files: 4),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: helps[each root names the module file, not just the option],
      names-reason: neutral,
      fast: hurts[one more `nix eval` of the configuration, about two seconds on tank],
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
  links: parent("name-the-option-and-module-file-requesting-each-root.typ"),
)

== Problem

+ Run `nix-why-build .#nixosConfigurations.tank`.
+ 🔴 `llama-cpp-0.5.0` is wanted by `environment.systemPackages`, which 90 modules set; nothing says which one lists it.


== TO-BE

- 🟢 One `nix eval` of the configuration answers every request: for a list option the definitions whose value holds the item's `drvPath`, for `systemd.*` the definitions setting that unit, for `environment.etc` and `home.file` the entry whose `source` is the item.
- 🟢 home-manager requests resolve to `home-manager.users.<u>.…` by the user whose `home.path` or `home-files` is the glue node.
- 🟢 Files print relative to their flake: `apps/llama-cpp-16gb.nix` for the configuration's own, `nixpkgs:nixos/modules/…` for an input's.
- 🟢 A failed evaluation leaves the files empty and says so once; the rest of the report stands.


== Changes

- `src/modules_tests.rs`: red tests first, the expression run by real `nix eval` against a hand-built configuration.
- `src/modules.rs`: the query, the Nix expression, the answer and the file labels.
- `src/eval.rs`: the configuration attribute behind an installable, `nix flake archive` sources.
- `src/lib.rs`: run it after the walk when the installable is a configuration.


== Alternatives

- `nix eval` per request: hundreds of evaluations of the same configuration.
- Matching by package name: two `python3` entries with different overrides would collide.


== Out of scope

- `users.users.<u>.packages`: it stays an option without a file until someone needs it.


== Dogfood

The tank report names `apps/llama-cpp-16gb.nix` for `llama-cpp-0.5.0`.

