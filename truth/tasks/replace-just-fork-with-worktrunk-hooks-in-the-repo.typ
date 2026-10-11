#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Replace just fork with worktrunk, hooks in the repo",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 6,
  ),
  difficulty: 2,
  tags: ("tooling",),
  estimate: (src-files: 7),
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
      sound-codebase: helps[a landing is gated by the repo, not by a convention],
      docs-match-code: helps[AGENTS.md and CONTRIBUTING.md name the commands that exist],
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 11),
)

== Problem

+ Run `just fork scratch`: it is `git worktree add -b fork/scratch ../nix-why-build-scratch HEAD` plus `direnv allow`, and nothing else.
+ Land the branch by hand with `git merge --ff-only fork/scratch`, with no gate and no `mt check --commits`.
+ 🔴 The merge goes in with a red or never-run `just check`, and with a commit that names a task still `wip`; nothing refuses it.
+ 🔴 The gate records in `.mmz/cache/` and the lcov under `target/coverage/` stay in the removed worktree, so `just check` in the primary re-runs every arm and `just crap` refuses its stale lcov.
+ 🔴 A clone without the maintainer's own `wt` hooks cannot reproduce the flow: it lives in a recipe nobody else's tooling knows.


== TO-BE

- 🟢 `just fork <slug> [wt flags]` makes a sibling worktree on a new branch with its dev shell allowed, from the dev shell's own `wt`.
- 🟢 `just land` in the worktree runs `just check`, then `mt check --commits main..HEAD`, fast-forwards main and removes the worktree.
- 🟢 The removal carries the gate records and a fresh lcov into the primary checkout.
- 🟢 A clone with nix and direnv needs no worktrunk config of its own: the hooks live in `.config/wt.toml`.


== Changes

- `tests/it/wt_carry_lcov.rs`: the red test first, the lcov carry copying a fresh lcov, rewriting `SF:` and skipping a stale or unlanded one.
- `scripts/wt-carry-lcov.sh`: the pre-remove lcov carry.
- `scripts/crap.sh`: add `--stale-reason`, which the carry asks.
- `.config/wt.toml`: `pre-start` direnv allow, `pre-merge` gate and `mt check --commits`, `pre-remove` mmz import and lcov carry.
- `flake.nix`, `flake.lock`: `worktrunk` in the dev shell; bump qahq for an mmz with `--import`.
- `Justfile`: `fork SLUG *ARGS` and `land *ARGS` over `wt`.
- `outdatty.yaml`, `outdatty.lock`: `.config/wt.toml` joins the dev-docs sources.
- `AGENTS.md`, `CONTRIBUTING.md`: name `just fork` and `just land`, and ban a bare `wt merge`.


== Rollback

- Revert the commits. `just fork` and `just land` come back as the old recipe, a plain worktree add.


== Out of scope

- A user-level worktrunk config. The point is that a clone needs none.
- Rebasing a branch that fell behind main. `wt merge` refuses it and a person rebases.


== Dogfood

This task lands through the new flow: `just fork scratch --base=@`, a commit, `wt hook pre-merge`, `wt remove`.

