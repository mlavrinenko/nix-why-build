#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Ship the dependencies' license notices with the release binaries",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 6,
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
      sound-codebase: helps[the binaries we distribute honor the licenses of the crates inside them],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 11),
)

== Problem

+ Run `nix build .#static` and look in `result/`, or open a release's assets.
+ 🔴 The binary bundles MIT and Apache-2.0 crates, and nothing ships their copyright and license notices, which both licenses ask of a copy.


== TO-BE

- 🟢 Every GitHub release carries a `THIRD-PARTY-LICENSES` asset beside the binaries, naming each crate built into them with its license text.
- 🟢 `just licenses` writes the same file locally from `Cargo.lock`, so the release step is reproducible.
- 🟢 A crate whose license cargo-about cannot resolve fails the release before anything is published.


== Changes

- `flake.nix`: `cargo-about` in the dev shell.
- `about.toml` and its template: the accepted licenses, matching `deny.toml`.
- `Justfile`: a `licenses` recipe.
- `.github/workflows/release.yml`: generate the file and attach it to the release.


== Rollback

Revert the commit; the release goes back to binaries only.


== Out of scope

- The Nix package's output. nixpkgs ships no notices for Rust binaries either, and `meta.license` covers the crate itself.
- The crates.io package, which ships source and no dependency code.


== Dogfood

Not applicable: the tool reads derivations, not license files.

