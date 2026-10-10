#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Print the report grouped by root",
  kind: "feat",
  priority: ice(
    impact: 9,
    confidence: 0.7,
    ease: 5,
  ),
  difficulty: 4,
  tags: ("report", "cli"),
  estimate: (src-files: 2),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: helps[the user reads a handful of roots, each with what it drags in],
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: helps[the README shows the grouped tank output],
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
  links: parent("group-misses-by-root-with-cascade-chains.typ"),
)

== Problem

+ Run `nix-why-build .#nixosConfigurations.tank`.
+ 🔴 It prints 45 packages flat, so the few that cause the rest drown in the list.


== TO-BE

- 🟢 Each root prints on its own line, a fetch marked as one, with the packages it drags in below it, nearest first and capped.
- 🟢 A root with no `pname` and nothing named below it (pure config glue) is counted, not printed.
- 🟢 `--json` carries every root with every cascade and its chain.


== Changes

- `tests/cli.rs`: red tests first, the grouped text and JSON.
- `src/report.rs`: roots with cascades in the report, grouped rendering.
- `src/lib.rs`: build the report from the split.


== Alternatives

- A tree view: a cascade under several roots would repeat, and deep chains wrap.


== Out of scope

- Who requested a root, and why it misses. Milestones 3 and 4.


== Dogfood

The README's sample is the tank output, pasted, trimmed only with `...`.

