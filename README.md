# nix-why-build

[![CI](https://github.com/mlavrinenko/nix-why-build/actions/workflows/ci.yml/badge.svg)](https://github.com/mlavrinenko/nix-why-build/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/crates/l/nix-why-build.svg)](LICENSE-MIT)

Run `nix run github:mlavrinenko/nix-why-build -- .#nixosConfigurations.mypc`
to list all packages that would need compiling. This tool was created to make
such packages visible, so you can act on them (find a cache, drop a local
patch, bump the package and so on).

Ready to install? Run `cargo install nix-why-build` or take the overlay from
`inputs.nix-why-build.overlays.default`, then use `pkgs.nix-why-build` in
`environment.systemPackages`. No cargo or flakes? Grab a binary from the
[releases page](https://github.com/mlavrinenko/nix-why-build/releases/latest).

## Output example

```console
$ nix-why-build .#nixosConfigurations.mypc
linux-UFRII-drv-v620-m17n-20.tar.gz (fetch)
  why fetch: no cache has https://gdlp01.c-wss.com/gds/8/0100007658/47/linux-UFRII-drv-v620-m17n-20.tar.gz
  wanted by systemd.services.cups (nixpkgs:nixos/modules/services/printing/cupsd.nix)
  drags in canon-cups-ufr2-6.20
llama-cpp-0.5.0
  why overridden: vs unstable#pkgsCuda.llama-cpp (cached): postPatch added; buildInputs, cmakeFlags changed; input blas-3 added
  wanted by environment.systemPackages (modules/llama.nix)
thunderbird-156.0
  why not-built-by-hydra: nixpkgs#thunderbird has meta.hydraPlatforms = [ ]
  wanted by home-manager.users.me.home.packages (home-manager:modules/programs/thunderbird.nix)
3 roots, 3 packages to build, 23 more without a pname
```

Each entry is a root, a package no cache has while everything it needs is
cached. Fix the root and everything it drags in comes from the cache too.

## Possible reasons

`why` compares a root with the same attribute in every nixpkgs input of your
flake.

| why | means |
| --- | --- |
| `local-only` | `allowSubstitutes = false`, so nix builds it even if a cache has it |
| `fetch` | a download no cache has, so the build fetches it |
| `vendored` | a crate of a `cargoLock.lockFile` package, or its fetch; `cargoHash` fetches them all in one |
| `overridden` | differs from the nixpkgs one, and the diff names what changed |
| `unfree` | same as in nixpkgs, but Hydra skips unfree packages |
| `not-built-by-hydra` | same as in nixpkgs, but `meta.broken` or `meta.hydraPlatforms` keeps Hydra off it |
| `newer-than-cache` | same as in nixpkgs, but the cache is behind, even its `hello` is missing |
| `no-counterpart` | no nixpkgs attribute guessed from its pname, nor any with that pname, and it lists what it tried |
| `unknown` | none of the above could be proven, and it says why |

## Possible targets

`nix-why-build` can take:

- a NixOS config, `.#nixosConfigurations.mypc`
- a home-manager config, `.#homeConfigurations.me`
- a nix-darwin config, `.#darwinConfigurations.mac`
- any other installable `nix build` takes, like `nixpkgs#hello` or `.`
- a `.drv` path, `/nix/store/...-foo.drv`

## Interop usage

Pass `--json` to get machine-readable data. See
[its schema](docs/report.schema.json).

## Library usage

```rust
use nix_why_build::{Options, explain};

let report = explain(&Options::new(".#nixosConfigurations.mypc")).await?;
for root in &report.roots {
    println!("{} {}", root.name, root.reason);
}
```

The rest is on [docs.rs](https://docs.rs/nix-why-build).

## Architecture

See [the docs](docs/architecture.md).

## Thanks

This tool stands on other people's crates. Big thanks to

- [nix-derivation](https://crates.io/crates/nix-derivation) and
  [nix-narinfo](https://crates.io/crates/nix-narinfo) by Cachix, which parse
  every `.drv` and check every narinfo signature. The walk is built on them.
- [tokio](https://crates.io/crates/tokio),
  [reqwest](https://crates.io/crates/reqwest) and
  [futures](https://crates.io/crates/futures), which ask all the caches at once.
- [serde](https://crates.io/crates/serde) and
  [serde_json](https://crates.io/crates/serde_json) for `--json`, and
  [schemars](https://crates.io/crates/schemars) for its schema.
- [clap](https://crates.io/crates/clap) for the flags and
  [anstyle](https://crates.io/crates/anstyle) for the colors.
- [thiserror](https://crates.io/crates/thiserror),
  [anyhow](https://crates.io/crates/anyhow),
  [log](https://crates.io/crates/log) and
  [env_logger](https://crates.io/crates/env_logger) for errors and logs.
- [assert_cmd](https://crates.io/crates/assert_cmd),
  [predicates](https://crates.io/crates/predicates),
  [tempfile](https://crates.io/crates/tempfile) and
  [ed25519-dalek](https://crates.io/crates/ed25519-dalek), which keep the
  tests honest.
