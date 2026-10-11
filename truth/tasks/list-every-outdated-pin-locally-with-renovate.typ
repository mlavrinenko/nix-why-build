#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "List every outdated pin locally with Renovate",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 3,
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
      sound-codebase: helps[a stale action, crate or flake input shows up before the push, not as a bot PR after it],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 11),
)

== Problem

+ Push a branch whose workflows pin an old action.
+ 🔴 The first word of it is a Dependabot PR on GitHub; nothing local lists outdated actions or flake inputs, and `just outdated` covers crates only.


== TO-BE

- 🟢 `just renovate` runs Renovate on the checkout with no push and no PR, and prints one line per update: manager, dependency, current version, proposed version.
- 🟢 It covers the workflows' actions, Cargo.toml and flake.lock.
- 🟢 A green run prints the list and nothing else; Renovate's own logs show only on failure.


== Changes

- `flake.nix`: `renovate` in the dev shell.
- `renovate.json`: the managers to scan, nix included.
- `Justfile` and `scripts/renovate.sh`: run Renovate in local dry-run mode and reduce its JSON log to the list.
- `CONTRIBUTING.md`: when to run it.


== Rollback

Revert the commit; `just outdated` and Dependabot remain.


== Out of scope

- Renovate as a GitHub app opening PRs; Dependabot stays the remote bot for now.
- Any gate: a new upstream release must never turn `just check` red.


== Dogfood

Not applicable: it watches this repo's pins, not derivations.

