#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Highlight the report on a terminal",
  kind: "feat",
  priority: ice(
    impact: 4,
    confidence: 0.7,
    ease: 6,
  ),
  difficulty: 3,
  tags: ("cli", "report"),
  estimate: (src-files: 2),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: helps[root names stand out from the lines under them],
      names-requester: neutral,
      names-reason: helps[the `why` category reads at a glance],
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Run `nix-why-build .#nixosConfigurations.tank` in `~/.nix`.
+ 🔴 135 lines of plain text: the root names, the `why` categories and the labels look the same, so finding the `overridden` roots among 39 groups means reading every line.


== TO-BE

- 🟢 On a terminal, root names are bold, the labels (`why`, `wanted by`, `drags in`) dim, and each category has its own color: `overridden` and `unknown` stand out, `fetch` and `local-only` recede.
- 🟢 `NO_COLOR`, a pipe or a file gets plain text, byte for byte what prints today.
- 🟢 `--color auto|always|never` overrides the guess, for `| less -R`.


== Changes

- `src/human.rs` tests: red test first. The styled rendering wraps a root name in bold; the plain one equals today's output.
- `src/human.rs`: render through `anstyle` styles, which clap already pulls in, so `Cargo.lock` gains no crate.
- `src/main.rs`: the `--color` flag and the terminal and `NO_COLOR` check.


== Alternatives

- `owo-colors` or `colored`: one more crate for what `anstyle` and `anstream` already do.


== Out of scope

- Colored `--json`: that is for `jq`.


== Dogfood

The tank report shows its two `overridden` roots at a glance.

