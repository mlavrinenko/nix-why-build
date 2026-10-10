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
  It is a root of its own with `why local-only` and makes no parent a
  cascade. `to_build` now equals nix's own dry-run plan on an empty store:
  1673 derivations on tank, where it had 1469.
- `just oracle <installable>` diffs `to_build` with `nix build --dry-run` in a
  scratch store holding only the derivation closure; it prints nothing when
  they agree.
- Every `nix` call enables `nix-command` and `flakes` itself, as
  `nixos-rebuild --flake` does, so a nix with them off in its configuration
  still works.
- `nix-why-build .` and any other installable `nix build` accepts work: nix
  resolves the `.drv` itself with `nix path-info --derivation`, so a flake
  reference without `#` explains its default package. A configuration with a
  quoted, dotted name (`.#nixosConfigurations."host.lan"`) resolves to its
  toplevel and gets its option index.
- A closed stdout ends the run quietly with exit 0, as `ripgrep` does, so
  `nix-why-build ... | head` no longer ends in `Error: Broken pipe`. Any other
  write error still fails.
- When `nix flake archive` fails, a warning says why (`warning: flake inputs
  unknown: error: boom`) and each root's `why unknown` reads `the flake inputs
  could not be listed` instead of blaming the flake for having no nixpkgs input.
- Local-only roots that drag in nothing named print as one group at the end of
  the report, `11 local-only: cuda-merged-12.9, gimp-with-plugins-3.0.8, ...`,
  with one `why` and the merged `wanted by`. On tank the report went from 135
  lines in 39 groups to 105 lines in 29. `--json` still lists every root.
- `wanted by` names each option once per group, with the files of every root
  that sets it: `environment.systemPackages (apps/audiocpp.nix,
  nixpkgs:nixos/modules/programs/steam.nix +3)` where the option used to
  fill both slots with a file each.
- A substituter nix-why-build cannot ask (`s3://`, `ssh-ng://`) is left out,
  and one it cannot reach (refused, no DNS, no answer twice) is disabled after
  its first failure, each with one warning; its paths count as misses, as nix
  treats a cache it cannot reach, so roots stay roots and a dead cache costs
  one wait per run.

### Changed

- The embedded Nix expression follows `nixfmt` layout, which `just check` now
  enforces; evaluation is unchanged.
- The nixpkgs lookup sends its query to nix on stdin, so a mass rebuild with
  thousands of roots keeps its reasons instead of failing every lookup with
  `Argument list too long`.
- The Linux release binaries are static musl builds from the flake
  (`nix build .#static`), so they start on NixOS, where the glibc binaries
  built on Ubuntu found no `/lib64/ld-linux`. The Windows binary is gone: nix
  does not run there.
- The library exports only `explain`, `Options`, `Error`, `Report` with its
  `Root`, `Package`, `Requested`, `Cascade` and `Unknown`, `Reason` and
  `Diff`. The modules behind them (`cache`, `walk`, `roots` and the rest) are
  private, and the structs and enums are `#[non_exhaustive]`, so a field or a
  category can be added without a breaking release. Build `Options` with
  `Options::new` plus `with_substituters`, `with_trusted_keys` and
  `with_store_dir`.
- On a terminal, one line on stderr says what the run is waiting on and
  rewrites itself per phase: `evaluating .#nixosConfigurations.tank`, then
  `asking 5 caches: level 7, 3210 derivations`, then `comparing 52 roots with
  nixpkgs`, and it is gone before the report prints. A pipe or a file gets
  not one extra byte. `RUST_LOG=info` logs each phase with its duration. As a
  library, `Options::with_progress` takes a callback that hears each
  `Progress` phase; the library prints nothing itself.
