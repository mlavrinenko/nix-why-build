#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Ask the remaining substituters at once after the first misses",
  kind: "feat",
  priority: ice(
    impact: 6,
    confidence: 0.8,
    ease: 7,
  ),
  difficulty: 3,
  tags: ("cache",),
  estimate: (src-files: 1),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: helps[a missing path costs two round trips, not one per substituter],
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
  links: parent(
    "list-uncached-packages-for-a-flake-configuration-store-independent.typ",
  ),
)

== Problem

+ Run `RUST_LOG=info nix-why-build /nix/store/rcfnhh5dg3n04xibyjgv5q84yps9ifcr-nixos-system-tank-26.05.20260927.cf5e765.drv` against tank's five substituters.
+ 🔴 The walk alone takes 23.6 s; one missing path costs five sequential round trips of 200 to 350 ms, so every level waits about 1.4 s on its slowest chain.


== TO-BE

- 🟢 The first substituter is asked alone, since it serves most paths; on a miss the rest are asked concurrently, and the first in configured order that serves the path wins.
- 🟢 The tank walk drops well below 23.6 s with the same output.


== Changes

- `src/cache_tests.rs`: the red test first, two caches that answer only once both were asked.
- `src/cache.rs`: probe the rest concurrently, keep configured order for the answer.


== Alternatives

- Ask every substituter at once: five times the requests for the paths `cache.nixos.org` serves, which are most of them.
- Only raise the concurrency cap: the per-path chain stays five round trips long.


== Out of scope

- Reading cache priority from `nix-cache-info`.


== Dogfood

Time the tank walk before and after; the README keeps the measured number honest.

