#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Rename the maintainer's host out of the tests and templates",
  kind: "dev",
  priority: ice(
    impact: 2,
    confidence: 0.8,
    ease: 9,
  ),
  difficulty: 1,
  tags: ("tests",),
  estimate: (src-files: 10),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: helps[tests, templates and rustdoc name the same `mypc` the README does],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 11),
)

== Problem

+ Run `rg -i tank src tests .mindtape`.
+ 🔴 41 lines name the maintainer's machine: `.#nixosConfigurations.tank`, `home-manager.users.tank`, `nixos-system-tank`, and every task template's example. The tests ship in the crate.


== TO-BE

- 🟢 Tests, fixtures and templates use `mypc` for the host and `me` for the user, as the README does.
- 🟢 The changelog rule takes a `Changelog: none` trailer, which mt 0.3.0 reads.


== Changes

- `src/eval.rs`, `src/modules.rs`, `src/progress.rs`, `src/terminal.rs`: inline tests and a doc comment.
- `src/*_tests.rs`, `tests/fixtures/config/flake.nix`: test data.
- `.mindtape/templates/*.typ`: the examples.
- `.mindtape/config.toml`: the changelog rule honors `Changelog: none`; its stale comment is gone.


== Rollback

- Revert the commit. Nothing user-facing changes.


== Out of scope

- Closed tasks and commit messages. They record what happened.


== Dogfood

Not applicable: test data only.

