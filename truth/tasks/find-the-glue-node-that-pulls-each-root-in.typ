#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Find the glue node that pulls each root in",
  kind: "feat",
  priority: ice(
    impact: 8,
    confidence: 0.7,
    ease: 5,
  ),
  difficulty: 5,
  tags: ("walk", "report"),
  estimate: (src-files: 3),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: helps[each root names the option kind that asked for it, before any evaluation],
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[requests are report data],
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
+ 🔴 `llama-cpp-0.5.0` prints as a root with nothing saying it comes in through `environment.systemPackages`.


== TO-BE

- 🟢 Walking up from a root, the first glue node on each way to the toplevel names an option: `system-path` is `environment.systemPackages`, `etc` is `environment.etc`, `unit-<n>.service` is `systemd.services.<n>` (or `systemd.user.services.<n>` under `user-units`), `home-manager-path` is `home.packages`, `home-manager-files` is `home.file`.
- 🟢 Each request keeps the item the option lists, the derivation right below the glue node.
- 🟢 A way that reaches the toplevel with no glue node says `toplevel`, nothing invented.


== Changes

- `src/requester_tests.rs`: red tests first, over the fake store.
- `src/requester.rs`: the upward search and the glue-name map.
- `src/report.rs`: `requested_by` per root, a `wanted by` line per group.


== Alternatives

- Only the shortest path from the toplevel: it hides a unit that runs the same package `environment.systemPackages` lists.


== Out of scope

- Module files. The next leaf evaluates them.


== Dogfood

The tank report says `llama-cpp-0.5.0` is wanted by `environment.systemPackages`.

