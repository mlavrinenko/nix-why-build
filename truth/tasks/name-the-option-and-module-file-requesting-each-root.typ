#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Name the option and module file requesting each root",
  priority: ice(
    impact: 8,
    confidence: 0.6,
    ease: 4,
  ),
  difficulty: 7,
  kind: "milestone",
  tags: ("eval", "report"),
  estimate: (src-files: 0),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: helps[each root names the option and the file that sets it],
      names-reason: neutral,
      fast: hurts[one more evaluation of the configuration],
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: wip(2026, 10, 10),
  links: depends-on("group-misses-by-root-with-cascade-chains.typ"),
)

== Problem

+ Run the milestone-2 build against tank.
+ 🔴 `llama-cpp-0.5.0` is a root, but nothing says which module asked for it; the user greps `~/.nix` by hand.


== TO-BE

- 🟢 Each root shows the path from the toplevel down to it, cut at the first glue node that maps to an option (`unit-<name>.service` → `systemd.services.<name>`, `system-path` → `environment.systemPackages`, `etc` → `environment.etc`, `home-manager-path` → `home.packages`).
- 🟢 The option's `definitionsWithLocations` names the module file(s); a root it cannot place says unknown.


== Changes

- `src/requester.rs`: the requester path and the glue-node map.
- `src/eval.rs`: one `nix eval` that returns the definitions and their files.
- `src/report.rs`: a `wanted by` line per root.


== Out of scope

- nix-darwin glue nodes beyond `system-path`. Filed when someone runs it on a Mac.


== Dogfood

The tank report says `llama-cpp-0.5.0` comes from `apps/llama-cpp-16gb.nix`.

