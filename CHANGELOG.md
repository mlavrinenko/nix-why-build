# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `cache::Substituters` asks binary caches, in parallel, whether they serve a
  store path, and counts a narinfo only when a trusted key signed it.
- `walk::walk` reads `.drv` files with `nix-derivation` and walks from a toplevel
  down to what substituters serve, asking only about the outputs a parent
  wants.
- `nix-why-build <installable>` prints every package no substituter serves,
  one per line, with a tally of hidden config glue on stderr; `--json` prints
  the whole report. NixOS, home-manager and nix-darwin configurations resolve
  to what they build. `explain()` is the same thing as a library call.
- After the first substituter misses, the rest are asked at once, and a
  request stalled past 1.5 s is sent again: the tank walk went from 23.6 s to
  about 9 s.
- `roots::split` tells roots (built while every input is served) from
  cascades (built because an input is), with a shortest chain from each
  cascade down to each of its roots.
- The report groups by root: each root, or each group of roots that drag in
  the same packages, prints with what it drags in. `--json` carries every
  root with every cascade and its chain.
- Each root says which options pull it in (`wanted by`): walking up from it,
  the first glue node on every way to the toplevel names one, like
  `environment.systemPackages` for `system-path` or `systemd.services.<n>`
  for `unit-<n>.service`.
- `wanted by` names the module files that set each option, from one more
  `nix eval` of the configuration that runs while the caches are asked:
  `apps/llama-cpp-16gb.nix` for the configuration's own, `<input>:path` for a
  flake input's. home-manager options resolve per user, units by their real
  name, `environment.etc` and `home.file` by the entry whose source it is.
- `diff::Diff` names what differs between a root and its nixpkgs counterpart:
  attributes added, removed and changed (structured attrs key by key, store
  hashes ignored), and inputs added, removed or built differently.
- `counterparts::find` looks roots up in every nixpkgs-like input of the
  configuration, under a few attribute guesses and `pkgsCuda` for CUDA roots,
  with each candidate's `meta.unfree`, `meta.broken` and
  `meta.hydraPlatforms`.
- Each root says `why` it misses: `fetch`, `overridden` (with the diff against
  the closest cached counterpart of its version), `unfree`,
  `not-built-by-hydra`, `newer-than-cache`, `no-counterpart` or `unknown`,
  each with its evidence. CUDA roots compare against `pkgsCuda`, 32-bit ones
  against `pkgsi686Linux`.
- Every dependency comes from crates.io under a permissive license: Cachix's
  Apache-2.0 `nix-derivation` and `nix-narinfo` parse `.drv` files and
  narinfos and verify cache signatures, so the crate stays MIT and publishes
  to crates.io.
- A derivation with `allowSubstitutes = false` counts as built, as nix builds
  it even when a cache serves it, and the walk decides its inputs as usual.
  Such a root says `why local-only`. `to_build` now equals nix's own dry-run
  plan on an empty store: 1673 derivations on tank, where it had 1469.
- `just oracle <installable>` diffs `to_build` with `nix build --dry-run` in a
  scratch store holding only the derivation closure; it prints nothing when
  they agree.
