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
  status: done(
    2026,
    10,
    10,
  )[every dependency is on crates.io; just release publishes],
  links: related(
    "decide-between-gpl-3-0-nix-compat-and-the-apache-2-0-cachix-parsers.typ",
  )[the parser switch is what opened crates.io],
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


== Decision

Release through `just release`, now that nothing is git-only. Switching to Cachix's `nix-derivation` and `nix-narinfo` (the related decision) left every dependency on crates.io.

- crates.io: `cargo publish --dry-run --allow-dirty --locked` ends in `aborting upload due to dry run`; `release.yml` publishes on the tag.
- GitHub releases: `release.yml` builds the binaries on the same tag.
- Flake: `flake.nix` drops the `nix-compat` `outputHashes` entry, so `nix build` vendors from `Cargo.lock` alone, and it builds.
- `README.md` and `www/index.html` install with `cargo install nix-why-build`.


== Changes

- `Cargo.toml`: whichever source the decision picks.


== Rollback

- Revert to the git pin.


== Out of scope

- Releasing before the feature milestones land.


== Dogfood

Not applicable: release plumbing, nothing the tank run exercises.

