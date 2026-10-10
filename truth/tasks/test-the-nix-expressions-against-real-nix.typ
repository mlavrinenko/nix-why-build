#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Test the Nix expressions against real nix",
  priority: ice(
    impact: 8,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 5,
  kind: "test",
  tags: ("tests", "eval"),
  estimate: (src-files: 1),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: helps[a broken `index.nix` fails a test instead of silently dropping every `wanted by` file],
      names-reason: helps[a broken `counterparts.nix` fails a test instead of turning every root unknown],
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: helps[the half of the product written in Nix gets the gate the Rust half has],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
)

== Problem

+ Read `src/index.nix` and `src/counterparts.nix`, then `tests/cli.rs` and the unit tests.
+ 🔴 Every test fakes `nix` with a shell script that prints canned JSON, so neither expression is ever evaluated by a gate. A typo, a renamed `meta` attribute or an option path nixpkgs moves passes `just check`; the first sign is a tank run where every root says `unknown` or no `wanted by` names a file.


== TO-BE

- 🟢 Tests evaluate both expressions with the real `nix` against small fixtures in `tests/`, offline and in well under a second each.
- 🟢 `counterparts.nix` is pinned on a fake nixpkgs (a `default.nix` taking `localSystem`, `config` and `overlays`, returning `builtins.derivation`s with `meta`): attribute guesses, `pkgsCuda` and `pkgsi686Linux` variants, `unfree`, `broken`, `hydraPlatforms`, an input that fails to import, and `hello`.
- 🟢 `index.nix` is pinned on a fake configuration with `options`, `definitionsWithLocations`, `systemd.services`, `environment.etc` sources and one home-manager user.
- 🟢 The tests write no `.drv` into the real store: they use a scratch store or a read-only eval.


== Changes

- `tests/nix_exprs.rs`: the tests, red against a deliberately broken copy of each expression before they go green.
- `tests/fixtures/`: the fake nixpkgs and configuration.
- `src/counterparts.rs`: expose the expression builder to integration tests only if it is not reachable already.


== Out of scope

- Evaluating real nixpkgs: slow and network-bound.


== Dogfood

`just check` runs these on every change to a `.nix` file in `src/`.

