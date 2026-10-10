#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Fold the local-only roots into one group",
  kind: "feat",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 8,
  ),
  difficulty: 2,
  tags: ("report",),
  estimate: (src-files: 1),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: helps[the report leads with roots a user can fix, not builds nix makes by design],
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: helps[the README example shows the folded group],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Run `nix-why-build .#nixosConfigurations.tank` in `~/.nix`.
+ 🔴 11 of the 39 groups are single local-only roots (`steam-1.0.0.85`, `nh-4.4.2`, `krita-6.0.2.1`, wrappers with `allowSubstitutes = false`), three lines each, all saying `why local-only: allowSubstitutes = false: nix builds it even when a cache has it`. That is 33 lines about builds nobody can turn into cache hits, spread among the roots that matter.


== TO-BE

- 🟢 Local-only roots that drag in no named package print as one group, the way fetches already do: `11 local-only: gimp-with-plugins-3.0.8, krita-6.0.2.1, lsb_release (+8 more)`, with one `why` and the merged `wanted by`.
- 🟢 A local-only root that drags in a named package keeps its own group.
- 🟢 `--json` is unchanged: every root is still listed with its own requesters.


== Changes

- `src/report_tests.rs` or the human tests: red test first. Two local-only roots that drag in nothing named print as one `2 local-only:` group.
- `src/human.rs`: group local-only roots with no named cascade together; the header says `local-only`.
- `README.md`: the usage example shows the folded group.


== Alternatives

- Hiding local-only roots entirely: the user would no longer see that `steam` is built locally on every switch.


== Out of scope

- The tally line: it counts roots, which stays true.


== Dogfood

The tank report loses 30 lines and leads with the roots worth fixing.

