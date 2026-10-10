#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Keep local-only builds out of cascades",
  kind: "dev",
  priority: ice(
    impact: 6,
    confidence: 0.9,
    ease: 8,
  ),
  difficulty: 3,
  tags: ("report", "found"),
  estimate: (src-files: 3),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: helps[a root nixpkgs overrides stays a root instead of hiding as a cascade of a local-only wrapper],
      names-requester: neutral,
      names-reason: helps[a local-only build is a root of its own that says why local-only],
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
)

== Problem

+ `nix-why-build .#nixosConfigurations.tank` from `~/.nix`, once the walk counts local-only derivations as built
+ 🔴 `src/roots.rs`: `llama-cpp-0.5.0` (overridden) prints as dragged in by `nodejs-26.10.0`, a local-only wrapper, because a built input makes its parent a cascade. Hydra builds and caches local-only derivations like any other, so they never explain a parent's miss.


== TO-BE

- 🟢 A local-only build is always a root, and makes no parent a cascade.
- 🟢 `wanted by` still names who pulls a local-only root in.
- 🟢 A local-only root with a pname is not looked up in nixpkgs, since its reason needs no counterpart.


== Changes

- `src/roots.rs`: split on the build inputs that miss, leaving local-only ones out.
- `src/walk.rs`: `Graph::local_only`.
- `src/lookup.rs`: no lookup key for a local-only root.


== Rollback

- Revert the commit; local-only builds turn back into cascade links.


== Out of scope

- Hiding local-only roots from the human report.


== Dogfood

Run `nix-why-build .#nixosConfigurations.tank` from `~/.nix`: `llama-cpp-0.5.0` is a root again.

