# nix-why-build

[![CI](https://github.com/mlavrinenko/nix-why-build/actions/workflows/ci.yml/badge.svg)](https://github.com/mlavrinenko/nix-why-build/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/nix-why-build.svg)](https://crates.io/crates/nix-why-build)
[![License: MIT](https://img.shields.io/crates/l/nix-why-build.svg)](LICENSE-MIT)

Explain why a Nix build compiles what no binary cache serves

## Install

### From crates.io

```bash
cargo install nix-why-build
```

### From binary releases

Download a pre-built binary from the
[latest release](https://github.com/mlavrinenko/nix-why-build/releases/latest).

## Usage

```bash
nix-why-build
```

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
