# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


### Added

- `nix-why-build <installable>` lists what a build compiles because no binary
  cache serves it. It asks the caches, not your store, so a box that built
  everything yesterday gets the same answer as a fresh one.
- It cuts the derivation graph where nix does on an empty store. `just oracle`
  checks that against `nix build --dry-run`, and CI runs it on every push.
- Derivations with `allowSubstitutes = false` count as built, since nix builds
  them whatever the cache holds.
- Misses group by root. A root is built while everything it needs is served,
  and the report shows what it drags in. Roots that drag in the same packages
  share one entry, and the local-only ones share one at the end.
- `wanted by` names the options that pull a root in
  (`environment.systemPackages`, `systemd.services.<name>`, `home.file` and
  friends) and the module files that set them, `apps/x.nix` for yours and
  `<input>:path` for a flake input's.
- `why` says why each root misses, with evidence, or says `unknown` and why.
  The categories are `local-only`, `fetch`, `overridden` with the attribute
  diff against nixpkgs, `unfree`, `not-built-by-hydra`, `newer-than-cache` and
  `no-counterpart`. CUDA roots compare against `pkgsCuda`, 32-bit ones against
  `pkgsi686Linux`, and a mass rebuild with thousands of roots still gets its
  reasons.
- A root whose attribute isn't its pname, like `nss-cacert` at
  `nixpkgs#cacert`, still finds its counterpart. Only a miss indexes the
  input by pname, once per nixpkgs revision, cached under
  `$XDG_CACHE_HOME/nix-why-build`.
- A `cargoLock.lockFile` package's crates and their fetches say `vendored`
  and share one entry, instead of hundreds of `unknown` and `fetch` roots.
  The fix is `cargoHash`, which fetches them all in one.
- Any installable `nix build` takes works, and so does a `.drv` path. NixOS,
  home-manager and nix-darwin configurations resolve to what they build,
  quoted dotted names included.
- Caches and keys come from `nix config show`, and `--substituter` and
  `--trusted-key` override them. A narinfo counts only when a trusted key
  signed it.
- A cache it can't speak to (`s3://`, `ssh-ng://`) is skipped, and one it
  can't reach is dropped after the first failure, each with one warning. Its
  paths count as misses, the way nix treats them.
- Caches are asked in parallel, a stalled request goes out again after 1.5 s,
  and the option index evaluates while the caches answer.
- On a terminal a progress line on stderr shows the phase it's in, and the
  report gets colors. `--color` and `NO_COLOR` decide the colors, and a pipe
  gets plain text either way.
- `--json` prints the whole report, every cascade with its chain down to its
  root. [docs/report.schema.json](docs/report.schema.json) is its JSON
  Schema, generated from the code.
- It runs on a nix with flakes switched off, and `| head` ends it quietly.
- As a library, `nix_why_build::explain` returns the same report. It prints
  nothing, takes an optional progress callback, and exports only what a caller
  needs, so it can grow without breaking you. A `Reason` prints as
  `category: evidence`.
- Linux release binaries are static, so they run on NixOS. The flake ships
  `overlays.default`, and `nix build .#static` reproduces the release binary.
- Every dependency is under a permissive license, so the crate stays MIT.
