# nix-why-build

[![CI](https://github.com/mlavrinenko/nix-why-build/actions/workflows/ci.yml/badge.svg)](https://github.com/mlavrinenko/nix-why-build/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/crates/l/nix-why-build.svg)](LICENSE-MIT)

Your `nixos-rebuild` is compiling Firefox again. Why? `nix build --dry-run`
won't tell you. It only lists what's missing from *your* store, so anything
you built yesterday quietly drops off the list.

`nix-why-build` asks the binary caches instead. It walks the derivation graph
from your system's toplevel and stops wherever a substituter serves the
output, the same cut nix makes on an empty store. Same answer on your laptop
and on a fresh box.

## Usage

```console
$ nix-why-build .#nixosConfigurations.tank
agent-browser-0.39.0
audio.cpp-542bb4e
canon-cups-ufr2-6.20
firefox-156.0.1
llama-cpp-0.5.0
nh-4.4.2
thunderbird-156.0
...
45 to build, plus 1424 config derivations
```

Packages go to stdout, one per line, so it pipes. The tally goes to stderr.
Config glue (units, `etc`, `system-path`, wrapper scripts) is cheap to build,
so it's counted, not listed.

It takes any of these.

```console
$ nix-why-build .#nixosConfigurations.tank      # builds config.system.build.toplevel
$ nix-why-build .#homeConfigurations.me         # builds activationPackage
$ nix-why-build .#darwinConfigurations.mac      # builds system
$ nix-why-build nixpkgs#hello                   # any installable
$ nix-why-build /nix/store/...-foo.drv          # or a .drv you already have
```

Caches and keys come from `nix config show`, `extra-substituters` included.
Flags override them.

```console
$ nix-why-build .#nixosConfigurations.tank \
    --substituter https://cache.nixos.org \
    --trusted-key cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
```

A narinfo only counts when a trusted key signed it, just like nix itself
(unless `require-sigs = false`).

Want it machine readable? Pass `--json`.

```json
{
  "toplevel": "/nix/store/rcfn...-nixos-system-tank-26.05.20260927.cf5e765.drv",
  "substituters": ["https://cache.nixos.org/", "https://cache.nixos-cuda.org"],
  "to_build": [
    {
      "name": "llama-cpp-0.5.0",
      "pname": "llama-cpp",
      "version": "0.5.0",
      "drv": "/nix/store/...-llama-cpp-0.5.0.drv"
    }
  ],
  "unknown": []
}
```

`unknown` lists derivations it couldn't decide, like a `.drv` it can't parse (a
floating content-addressed one) or a cache that errored instead of answering.

## As a library

```rust
let options = nix_why_build::Options::new(".#nixosConfigurations.tank");
let report = nix_why_build::explain(&options).await?;
for package in report.to_build.iter().filter(|p| p.pname.is_some()) {
    println!("{}", package.name);
}
```

## Install

```bash
cargo install --git https://github.com/mlavrinenko/nix-why-build
```

Not on crates.io yet, since it depends on snix's `nix-compat`, which lives on git only.

## Development

Prerequisites: [Nix](https://nixos.org/) with flakes enabled.

```bash
direnv allow         # or: nix develop
just install-hooks   # pre-commit hook running the (memoized) gate

just check           # fmt, clippy, tests, unused deps, file size, drift, copy-paste, tasks
just deny            # cargo-deny: advisories, licenses, bans (needs network)
just build
just test
just cover           # code coverage via cargo-llvm-cov (70% minimum)
just fmt             # format code
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions.

## License

MIT
