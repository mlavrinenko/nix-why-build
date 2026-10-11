#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Bring MindTape back into the CI dev shell",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 3,
  tags: ("tooling",),
  estimate: (src-files: 3),
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
      sound-codebase: helps[CI validates the backlog again, not only the local hooks],
      docs-match-code: neutral,
    ),
  ),
  assignee: "max",
  status: proposed(2026, 10, 11),
  start: on(2026, 11, 11),
)

== Problem

+ Push a commit with `--no-verify` that breaks a task file.
+ 🔴 CI stays green: `just check-tasks` skips itself when `CI` is set, because the dev shell has no MindTape and `github:mlavrinenko/mindtape` is far behind the CLI the repo uses.


== TO-BE

- 🟢 The dev shell carries `mt` from a mindtape flake input, so `just check-tasks` runs on CI like it does locally.
- 🟢 `just check-tasks` no longer skips itself on CI.


== Changes

- `flake.nix`: a `mindtape` input, its package in the dev shell, exposing `mt`.
- `Justfile`: drop the skip in `check-tasks`.


== Rollback

Put the `CI` skip back in `check-tasks`.


== Out of scope

- Publishing mindtape itself; that happens in its own repo first.


== Dogfood

Not applicable: this is CI plumbing.

