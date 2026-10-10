#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Build the flake package with the git dependency",
  kind: "fix",
  priority: ice(
    impact: 6,
    confidence: 0.9,
    ease: 7,
  ),
  difficulty: 2,
  tags: ("tooling",),
  estimate: (src-files: 1),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[nixpkgs-style packaging is how nh or a NixOS module would consume it],
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

+ Run `nix build .#default`.
+ 🔴 naersk fails: `error inheriting rust-version from workspace root manifest's workspace.package.rust-version` in the vendored `nix-compat` git checkout.


== Environment

- `main` at the commit that filed this, Nix 2.34.8, naersk from `flake.lock`, on Linux.


== TO-BE

- 🟢 `nix build .#default` builds the binary, with the `nix-compat` git source pinned by an output hash.


== Changes

- `flake.nix`: `rustPlatform.buildRustPackage` with `cargoLock.outputHashes` in place of naersk, which cannot resolve a git dependency's workspace-inherited fields.


== Out of scope

- Publishing to crates.io. Blocked until `nix-compat` is published there.


== Dogfood

`nix run . -- .#nixosConfigurations.tank` from `~/.nix` works like the dev build.

