#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Show progress while it works",
  kind: "feat",
  priority: ice(
    impact: 7,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 5,
  tags: ("cli", "api"),
  estimate: (src-files: 5),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: helps[the wait is visible and each phase's share of it shows],
      embeddable: helps[`nh` gets the phases through the library and draws them its own way],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
)

== Problem

+ Run `nix-why-build .#nixosConfigurations.tank` in `~/.nix`.
+ 🔴 Nothing prints for 37 s: 22 s of nix evaluation, 7 s of walk, 8 s comparing roots with nixpkgs. It looks hung, and the one place timings show is `RUST_LOG=info`, which logs walk levels only.


== TO-BE

- 🟢 When stderr is a terminal, one line on it is rewritten in place per phase: `evaluating .#nixosConfigurations.tank`, `asking 5 caches: level 7, 3210 derivations`, `comparing 52 roots with nixpkgs`, then cleared before the report prints.
- 🟢 When stderr is not a terminal (CI, `2> file`) nothing extra prints, so scripts see what they see today.
- 🟢 The library prints nothing itself: `explain` reports each phase to an optional callback the caller passes, and the CLI renders it.
- 🟢 `RUST_LOG=info` logs every phase with its duration.


== Changes

- `tests/cli.rs`: red test first. A piped stderr carries no progress line and no carriage return.
- `src/lib.rs`: a `Progress` stage type and the optional callback; report each phase.
- `src/walk.rs`: report each level.
- `src/lookup.rs`: report the nixpkgs comparison.
- `src/main.rs`: render the line on a terminal, clear it before the report.


== Alternatives

- `indicatif`: a spinner is all this needs, and a hand-rolled carriage-return line costs no dependency.
- Progress through `log` records: the CLI would parse log text, and `nh` could not tell a phase from a debug line.


== Out of scope

- Percentages or ETAs: the walk does not know its depth ahead of time.


== Dogfood

The tank run shows which phase it is in, and a slow cache shows up as a level that sits.

