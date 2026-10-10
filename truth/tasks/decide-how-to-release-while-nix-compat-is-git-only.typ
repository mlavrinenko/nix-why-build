#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Decide how to release while nix-compat is git-only",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.9,
    ease: 4,
  ),
  difficulty: 4,
  estimate: (src-files: 1),
  tags: ("found", "tooling", "decision"),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[nh can only depend on a crate it can fetch reproducibly],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "max",
  status: proposed(2026, 10, 10),
)

== Problem

+ Adding nix-compat for the cache and walk leaves
+ 🔴 #raw("Cargo.toml"): the crate depends on nix-compat by git, so \`cargo publish\` and \`just release\` refuse it

```
$ curl -s https://crates.io/api/v1/crates/nix-compat | jq .crate.max_version
"0.0.0-pre"
```
crates.io refuses any package with a git dependency; `nix-compat` on crates.io is an empty placeholder.


This fell outside the task that surfaced it, so it is filed here rather than
left in a closing report no one rereads. It belongs in
#raw("Cargo.toml").

== TO-BE

- 🟢 A release path exists: snix publishes `nix-compat`, or this crate vendors the few parsers it uses, or releases ship as git tags and Nix packages only.


== Changes

- `Cargo.toml`: whichever source the decision picks.


== Rollback

- Revert to the git pin.


== Out of scope

- Releasing before the feature milestones land.


== Dogfood

Not applicable: release plumbing, nothing the tank run exercises.

