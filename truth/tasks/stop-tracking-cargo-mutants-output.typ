#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Stop tracking cargo-mutants output",
  kind: "dev",
  priority: ice(
    impact: 3,
    confidence: 0.8,
    ease: 9,
  ),
  difficulty: 3,
  tags: ("tooling", "found"),
  estimate: (src-files: 1),
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
      sound-codebase: helps[a mutants run no longer dirties the tree],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ `cargo mutants --in-diff <diff>`
+ 🔴 `git status --short | wc -l` prints 435: `mutants.out` and `mutants.out.old` have been tracked since the first commit.


== TO-BE

- 🟢 A mutants run leaves `git status` clean.


== Changes

- `.gitignore`: ignore `/mutants.out` and `/mutants.out.old`; untrack both.


== Rollback

- Revert the commit.


== Out of scope

- The mutants config: only where its output lands in git changes.


== Dogfood

Not applicable: a repo hygiene change.

