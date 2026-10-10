#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Gate locally what CI gates",
  kind: "dev",
  priority: ice(
    impact: 7,
    confidence: 0.9,
    ease: 6,
  ),
  difficulty: 2,
  tags: ("tooling",),
  estimate: (src-files: 6),
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
      sound-codebase: helps[a fresh `just check` means CI goes green, and coverage cannot rot by a quarter unnoticed],
      docs-match-code: helps[rustdoc links and the AGENTS.md recipe list stay true],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Run `just cover`: lines are 96.66% covered, against a floor of 70%. Coverage can lose a quarter of the suite before anything fails.
+ 🔴 `just check` and the pre-commit hook leave out coverage and CRAP, so a commit with fresh gates can still turn CI red.
+ 🔴 Nothing checks rustdoc (`docs.rs` renders it), the formatting of the `.nix` files (product code, via `include_str!`), or the workflows in `.github/`: a typo in `release.yml` surfaces on release day.
+ 🔴 `AGENTS.md` says coverage runs in `just validate`, a recipe this Justfile does not have.


== TO-BE

- 🟢 `just check` gains gate-tagged arms: `coverage` (`cover`, then `crap`, as one memoized rule), `doc` (`cargo doc --no-deps` with `-D warnings`), `nix-fmt` (`nixfmt --check` over every tracked `.nix`) and `actionlint` over `.github/workflows`.
- 🟢 `just cover` fails under 95% of lines.
- 🟢 CI's separate coverage job is gone: `just check` covers it.
- 🟢 `AGENTS.md`, `CONTRIBUTING.md` and `README.md` describe the gate as it now is.


== Changes

- `Justfile`: the new recipes, the floor at 95, the `check` arms.
- `.mmz/config.yaml`: one gate-tagged rule per new arm, with a `nix` scope and a `workflows` scope.
- `flake.nix`: `nixfmt` and `actionlint` in the dev shell.
- `.github/workflows/ci.yml`: drop the coverage job.
- `src/index.nix`, `src/counterparts.nix`: whatever `nixfmt` reformats.


== Rollback

- Revert the commit; the gate shrinks back to today's arms and CI regains its coverage job.


== Out of scope

- Mutation testing in the gate: a branch sweep takes minutes per change.
- `cargo deny`: it needs network, so it stays in its CI job.


== Dogfood

Every later task in this repo lands through the wider gate, and the pre-commit hook refuses a commit that would fail CI's check job.

