#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Adopt MindTape as the task tracker",
  kind: "dev",
  priority: ice(
    impact: 6,
    confidence: 0.9,
    ease: 7,
  ),
  difficulty: 2,
  tags: ("tooling",),
  estimate: (src-files: 4),
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
      sound-codebase: helps[a malformed task fails `just check` like any other gate],
      docs-match-code: helps[the backlog says what each change is for, next to the code],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Run `ls truth` in a fresh clone.
+ 🔴 It prints `No such file or directory`: the milestones live in a chat log, not the repo.


== TO-BE

- 🟢 `mt ls` lists the backlog, and `just check` validates it with every other gate.
- 🟢 A commit names its task in a `Refs:` footer, and the `commit-msg` hook refuses one that does not.


== Changes

- `.mindtape/config.toml`: statuses, ICE priority, difficulty, src-files estimate, kind, assignee, WIP caps, goal check, close gate, views, commit rules.
- `.mindtape/templates/*.typ`: task, feat, fix, dev, docs, finding, tweak, with this repo's goals.
- `truth/lib.typ`, `truth/goals.typ`, `truth/process-goals.typ`: the goal registries the check reads.
- `Justfile`, `.mmz/config.yaml`: `just check-tasks` as a memoized gate arm.
- `.githooks/commit-msg`: `mt check --commit-msg`.


== Rollback

- Revert the commit. No crate code depends on it.


== Out of scope

- A sweep script filing tasks from coverage or size signals. Nothing here produces those yet.


== Dogfood

This repo's own milestones are filed and closed through it from the first feature on.

