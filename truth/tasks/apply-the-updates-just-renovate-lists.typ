#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Apply the updates just renovate lists",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 3,
  tags: ("tooling",),
  estimate: (src-files: 4),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: helps[every pin is current, so the next just renovate lists only what moved since],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: wip(2026, 10, 11),
)

== Problem

+ Run `just renovate`.
+ 🔴 It lists seven updates: base64 0.23, ed25519-dalek 3, serde_json 1.0.154, nixpkgs and qahq in flake.lock, download-artifact 8.0.2 and upload-artifact 7.0.2.


== TO-BE

- 🟢 `just renovate` prints `everything is current`.
- 🟢 `just check`, `nix flake check` and `just oracle .#default` stay green.


== Changes

- `.github/workflows/release.yml`: the two artifact actions.
- `Cargo.toml`, `Cargo.lock`, `tests/support/fixture.rs`: the crates, and the fixture signer if the majors moved its API.
- `flake.lock`: nixpkgs and qahq.


== Rollback

Revert the commits; each kind of pin lands in its own.


== Out of scope

- `dtolnay/rust-toolchain`, which Renovate cannot see; its SHA pin has no version comment.


== Dogfood

Not applicable: the updates are this repo's own pins.

