#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Say why the flake inputs could not be listed",
  kind: "fix",
  priority: ice(
    impact: 3,
    confidence: 0.9,
    ease: 9,
  ),
  difficulty: 1,
  tags: ("eval", "report"),
  estimate: (src-files: 2),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[an unknown reason points at the real failure instead of blaming the flake],
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

+ Make `nix flake archive --json --dry-run <flake>` fail, say with an input that cannot be fetched offline, and run `nix-why-build <flake>#nixosConfigurations.<host>`.
+ 🔴 `src/lib.rs` drops that error (`modules::labels(flake).await.ok()`). Every root says `why unknown: no nixpkgs flake input to compare against`, module files print as raw store paths, and no warning says what failed. The option index failing gets a warning; this sibling does not.


== Environment

- `nix-why-build` built from `main` at `57f2260`, Nix 2.34.8, on NixOS x86_64-linux.


== TO-BE

- 🟢 A failed `nix flake archive` adds one warning with nix's first error line: `warning: flake inputs unknown: <error>`.
- 🟢 Each root's `why unknown` says the flake inputs could not be listed, not that the flake has no nixpkgs input.


== Changes

- `tests/cli.rs`: red test first. A fake `nix` whose `flake archive` exits 1 with `error: boom`; stderr carries `warning: flake inputs unknown: error: boom`.
- `src/lib.rs`: keep the error and push the warning.
- `src/lookup.rs`: take the reason the inputs are missing, for the `unknown` line.


== Out of scope

- Retrying the archive.


== Dogfood

An offline tank run says why every root is unknown instead of blaming the flake.

