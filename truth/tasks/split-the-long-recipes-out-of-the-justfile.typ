#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Split the long recipes out of the Justfile",
  kind: "dev",
  priority: ice(
    impact: 4,
    confidence: 0.8,
    ease: 7,
  ),
  difficulty: 2,
  tags: ("tooling",),
  estimate: (src-files: 9),
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
      sound-codebase: helps[a recipe reads in one screen, and a script can be shellchecked],
      docs-match-code: helps[`just --list` says what each recipe does],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 11),
)

== Problem

+ Run `just --list`.
+ 🔴 It lists 35 recipes, many described by the last line of a long comment: `check  # `gate` tag live in .mmz/config.yaml.`
+ Open `Justfile`.
+ 🔴 520 lines: `release` alone is 87 lines of bash, and most recipes carry a paragraph of history above them.


== TO-BE

- 🟢 `just` lists the daily recipes, `just --list` lists all of them in groups, each with a one-line `[doc()]`.
- 🟢 No recipe body runs past about five lines; longer ones live in `scripts/`.
- 🟢 A green `just test` prints its summary line; a green `just coverage`, `machete` or `check-dry` prints nothing.


== Changes

- `Justfile`: `[doc()]` and `[group()]` on every recipe, comments cut to the core, bodies over five lines moved out.
- `scripts/cover.sh`, `scripts/crap.sh`, `scripts/eject.sh`, `scripts/oracle.sh`, `scripts/release.sh`: the moved bodies.
- `scripts/quiet.sh`: run a command and print its output only when it fails, for the recipes above.
- `.mmz/config.yaml`: `scripts/**` joins the inputs every gate shares.
- `outdatty.yaml`: `scripts/**` is tracked and couples to the dev docs.
- `.config/nextest.toml`: drop the stale duplicate comment line.
- `CONTRIBUTING.md`, `AGENTS.md`: point at `just` and the groups.


== Rollback

- Revert the commit. The recipe names and arguments stay the same.


== Out of scope

- A shellcheck gate over `scripts/`. Its own task once the scripts settle.


== Dogfood

Every gate run goes through the new recipes.

