#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Ask substituters whether a store path is served",
  kind: "feat",
  priority: ice(
    impact: 8,
    confidence: 0.8,
    ease: 6,
  ),
  difficulty: 4,
  tags: ("cache",),
  estimate: (src-files: 3),
  x-goals: (
    product: (
      store-independent: helps[the answer comes from narinfo files, not the local store],
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: helps[requests run in parallel over shared connections],
      embeddable: helps[an async library call, no subprocess],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
  links: parent(
    "list-uncached-packages-for-a-flake-configuration-store-independent.typ",
  ),
)

== Problem

+ Run `nix path-info --store https://cache.nixos.org <path>` once per substituter, as `~/.nix/scripts/to-build.sh` does.
+ 🔴 A desktop config takes 2m15s, most of it process start-up and serial round trips.


== TO-BE

- 🟢 `cache::Substituters::query(paths)` answers served, missing or unknown for each path, asking substituters in order and stopping at the first that serves it.
- 🟢 A narinfo counts only when a trusted key signed it, unless `require-sigs` is off.
- 🟢 `http(s)://` and `file://` substituters work; any other scheme is unknown.


== Changes

- `src/cache_tests.rs`: red tests first, against `file://` caches and a local HTTP server.
- `src/cache.rs`: the client, signature check and fan-out.
- `Cargo.toml`: `nix-compat` (git, pinned), `reqwest`, `tokio`, `futures`.


== Alternatives

- `nix-compat` from crates.io: the published crate is an empty `0.0.0-pre` placeholder, so it comes from snix's git, pinned by rev.
- `nix-rs-compat`, a fresh fork of it on crates.io: 55 downloads, one author; not worth the supply-chain bet.
- HEAD requests: cheaper, but the signature lives in the body, and an unsigned path would read as served.


== Out of scope

- Reading `nix config show`. The next leaf.
- Cache priority from `nix-cache-info`. Config order is what nix tries first too.


== Dogfood

Not applicable alone; the walk leaf runs it against tank.

