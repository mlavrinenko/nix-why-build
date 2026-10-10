#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Run the oracle in CI on our own package",
  kind: "dev",
  priority: ice(
    impact: 7,
    confidence: 0.8,
    ease: 7,
  ),
  difficulty: 2,
  tags: ("tooling", "walk"),
  estimate: (src-files: 2),
  x-goals: (
    product: (
      store-independent: helps[every push checks the walk against nix's own empty-store plan],
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: helps[a walk regression fails CI instead of waiting for someone to remember `just oracle`],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
)

== Problem

+ Read `AGENTS.md`: after touching `src/walk.rs` or `src/drv.rs`, run `just oracle <installable>` on a real configuration.
+ 🔴 Nothing runs it: it lives in a rule an agent or a human must remember, and a walk change that drifts from nix's planner passes every gate.
+ Run `just oracle .#default` in this repo: it printed nothing in 18 s, so our own package is a pinned, real graph (stdenv, vendored crates, the crate itself) the oracle can check anywhere.


== TO-BE

- 🟢 CI runs `just oracle .#default` on every push, in its own job; a disagreement with `nix build --dry-run` fails it with the one-sided lists.
- 🟢 The job also fails when `to_build` is empty, so an oracle that compares two empty lists cannot pass by accident.
- 🟢 `AGENTS.md` says CI runs the oracle on our package and a real configuration is still worth a manual run.


== Changes

- `Justfile`: the oracle refuses an empty `to_build` with a clear line.
- `.github/workflows/ci.yml`: an `oracle` job in the dev shell.


== Rollback

- Revert the commit; the oracle goes back to a manual step.


== Out of scope

- Running it on a NixOS configuration in CI: evaluating one costs minutes and a fixture system to maintain.


== Dogfood

The tool checks its own package build against nix on every push.

