#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Count an unusable substituter as serving nothing, as nix does",
  kind: "fix",
  priority: ice(
    impact: 7,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 5,
  tags: ("cache",),
  estimate: (src-files: 2),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: helps[a miss stays a root instead of turning unknown and stopping the walk],
      names-requester: neutral,
      names-reason: helps[roots keep their reasons instead of hiding behind one cache's failure],
      fast: helps[a dead cache costs one timeout per run, not one per walk level],
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

+ Run a probe `.drv` no cache has with `--substituter https://cache.nixos.org --substituter s3://bucket` and the cache.nixos.org key.
+ 🔴 It printed `0 roots, ..., 1 unknown (first: ...: s3://bucket: unsupported substituter scheme)`; without `s3://` it prints `1 root`. One `s3://`, `ssh-ng://`, `daemon` or `local` entry in `nix config show` turns every miss into unknown and stops the walk there.
+ Run `nixpkgs#hello`'s `.drv` with `--substituter http://10.255.255.1` (it never answers) first.
+ 🔴 One path took 14 s: 1.5 s, then 12 s, per walk level. Tank has 14 levels, and each miss still ends unknown. Nix disables a cache it cannot reach and builds.


== Environment

- `nix-why-build` built from `main` at `57f2260`, Nix 2.34.8, on NixOS x86_64-linux.


== TO-BE

- 🟢 A substituter whose scheme `nix-why-build` cannot ask is left out up front, with one warning: `warning: s3://bucket: unsupported scheme, not asked`.
- 🟢 A substituter that cannot be reached (connection refused, DNS failure, no answer twice) is disabled for the rest of the run after its first such failure, as nix disables a binary cache it cannot reach. One warning names it and the error; its probes count as misses, and no level waits on it again.
- 🟢 A wrong answer for one path (HTTP 5xx, a bad narinfo) still makes that path unknown.
- 🟢 `Report.substituters` lists only the substituters that were asked.


== Changes

- `src/cache_tests.rs`: red tests first. An `s3://` entry next to a file cache leaves a missing path `Missing`; a refused connection (a port bound, then released) disables its substituter after one failure, and later paths are not sent to it.
- `src/cache.rs`: split unsupported entries off in `Substituters::new`; a shared down-marker per substituter, set on a connection-level failure; an accessor for the warnings.
- `src/lib.rs`: copy the substituter warnings into `Report.warnings`.


== Out of scope

- Asking `s3://` or `ssh-ng://` stores through `nix path-info --store`: a feature for whoever uses one.
- Retrying a 5xx: nix's `download-attempts` is a separate question.


== Dogfood

A tank run keeps reporting roots while a cachix cache is down or the VPN to a private cache is off.

