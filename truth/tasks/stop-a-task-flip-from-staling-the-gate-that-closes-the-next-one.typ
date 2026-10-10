#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Stop a task flip from staling the gate that closes the next one",
  kind: "dev",
  priority: ice(
    impact: 4,
    confidence: 0.9,
    ease: 7,
  ),
  difficulty: 3,
  estimate: (src-files: 2),
  tags: ("found", "tooling"),
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
      sound-codebase: helps[closing tasks needs no ritual re-run of `just check`],
      docs-match-code: neutral,
    ),
  ),
  assignee: "max",
  status: proposed(2026, 10, 10),
)

== Problem

+ Closing a leaf and its milestone in one \`mt done\` call
+ 🔴 #raw(".mmz/config.yaml"): \`just check-tasks\` is gate-tagged and reads \`truth/\*\*\`, so every flip stales it and a two-task \`mt done\` fails on the second

```
$ mt done print-the-report-grouped-by-root group-misses-by-root-with-cascade-chains
flipped print-the-report-grouped-by-root: wip → done
mmz: `just check-tasks` is stale (inputs changed since it last passed)
error: group-misses-by-root-with-cascade-chains: gate "mmz --is-fresh --tag gate" failed (exit 1)
```
MindTape's own `.mmz/config.yaml` keeps `truth/tasks/**` out of every gate scope for this reason and validates tasks in the pre-commit hook instead.


This fell outside the task that surfaced it, so it is filed here rather than
left in a closing report no one rereads. It belongs in
#raw(".mmz/config.yaml").

== TO-BE

- 🟢 `mt done leaf milestone` closes both after one `just check`, and a malformed task still cannot be committed.


== Changes

- `.mmz/config.yaml`: drop the `gate` tag from `just check-tasks`.
- `.githooks/pre-commit`: run `mt check -q` beside the freshness assertion.


== Rollback

- Restore the tag and the hook line.


== Out of scope

- The commit-msg rules, which already run on every commit.


== Dogfood

The next milestone close in this repo is one `mt done` call.

