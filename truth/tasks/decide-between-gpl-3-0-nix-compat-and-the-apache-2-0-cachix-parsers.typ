#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Decide between GPL-3.0 nix-compat and the Apache-2.0 cachix parsers",
  kind: "dev",
  priority: ice(
    impact: 8,
    confidence: 0.9,
    ease: 4,
  ),
  difficulty: 5,
  estimate: (src-files: 4),
  tags: ("found", "decision"),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[nh is EUPL-1.2; linking GPL-3.0 code into it forces the whole binary's license],
    ),
    process: (
      sound-codebase: helps[`just deny` goes green without an exception],
      docs-match-code: neutral,
    ),
  ),
  assignee: "max",
  status: proposed(2026, 10, 10),
  links: related(
    "decide-how-to-release-while-nix-compat-is-git-only.typ",
  )[both are about the nix-compat dependency],
)

== Problem

+ Running \`just deny\` after the reason milestone
+ 🔴 #raw("Cargo.toml"): nix-compat is GPL-3.0 (snix's LICENSE) with no license field, while this crate is MIT

```
$ just deny
error[unlicensed]: nix-compat = 0.1.0 is unlicensed
advisories ok, bans ok, licenses FAILED
$ head -2 snix/LICENSE
                    GNU GENERAL PUBLIC LICENSE
                       Version 3, 29 June 2007
```
On crates.io, `nix-derivation` 0.6.1 and `nix-narinfo` 0.4.0 are Apache-2.0, published by Cachix, and cover the two formats this crate parses (`.drv` ATerm and narinfo with signatures).


This fell outside the task that surfaced it, so it is filed here rather than
left in a closing report no one rereads. It belongs in
#raw("Cargo.toml").

== TO-BE

- 🟢 Either the crate moves to GPL-3.0 knowingly, or `src/drv.rs`, `src/cache.rs` and the test fixture switch to the Apache-2.0 parsers and `just deny` passes with the allow-list as is.


== Changes

- `Cargo.toml`: the chosen parser crates.
- `src/drv.rs`, `src/cache.rs`, `src/diff.rs`: the parsing calls, if the parsers change.


== Rollback

- Revert to the pinned nix-compat.


== Out of scope

- Releasing. The git-only release question is its own task.


== Dogfood

`just deny` in this repo.

