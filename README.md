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
246 fetches: acorn-8.18.0.tgz, aix-ppc64-0.27.7.tgz, aix-ppc64-0.28.2.tgz (+243 more)
  why fetch (246)
  wanted by home-manager.users.tank.xdg.configFile."opencode/plugins/octrimmer.js" (apps/agents/opencode/plugins/octrimmer/octrimmer.nix)
  drags in octrimmer-0.1.1-sources, octrimmer-0.1.1
russian-trusted-ca-constrained.crt
  why unknown: it has no pname to look up in nixpkgs
  wanted by home-manager.users.tank.home.packages (home-manager:modules/programs/firefox), home-manager.users.tank.home.file.".mozilla/firefox/974ct1mu.default/search.json.mozlz4" (home-manager:modules/programs/firefox) (+1 more)
  drags in firefox-156.0.1
...
llama-cpp-0.5.0
  why overridden: vs unstable#pkgsCuda.llama-cpp (cached): postPatch added; buildInputs, cmakeFlags changed; input blas-3 added
  wanted by environment.systemPackages (apps/llama-cpp-16gb.nix), systemd.services.llama-cpp-embed (apps/llama-cpp-16gb.nix, base:apps/gpu-exclusive.nix) (+1 more)
nh-4.4.2
  why not-built-by-hydra: nixpkgs#nh has meta.hydraPlatforms = [ ]
  wanted by environment.systemPackages (nixpkgs:nixos/modules/programs/nh.nix), systemd.services.nh-clean (nixpkgs:nixos/modules/programs/nh.nix)
514 roots in 29 groups, 45 packages to build, 1424 more without a pname
```

A **root** is something no cache serves even though everything it needs is
served. That's where the miss starts. Fix the root and everything it
**drags in** comes from the cache again. Roots that drag in the same packages
print as one group, so a custom package's 223 vendored crates are one line.
Config glue (units, `etc`, `system-path`) is cheap, so it's counted, not
listed.

**wanted by** names the options that pull a root in and the module files that
set them. Yours print as `apps/llama-cpp-16gb.nix`, a flake input's as
`home-manager:modules/...`. When a module only mentions a package (an `etc`
file, a profile script), that's what you get. It shows where the reference
is, not why someone wrote it.

**why** compares the root with what nixpkgs builds under the same attribute,
in every nixpkgs input of your flake (and `pkgsCuda` when it uses CUDA).

| why | means |
| --- | --- |
| `fetch` | a fixed-output download no cache has; the build fetches it |
| `overridden` | differs from nixpkgs' own; the diff names the attributes and inputs |
| `unfree` | identical to nixpkgs', which Hydra skips as unfree |
| `not-built-by-hydra` | identical, but `meta.broken` or `meta.hydraPlatforms` keeps Hydra off it |
| `newer-than-cache` | identical, and that nixpkgs' `hello` isn't cached either |
| `no-counterpart` | no nixpkgs attribute matches its pname (it says what it tried) |
| `unknown` | none of the above could be shown, and it says why |

It never guesses. No evidence, no category.

It takes any installable. Configurations resolve to what they build.

```console
$ nix-why-build .#nixosConfigurations.tank      # config.system.build.toplevel
$ nix-why-build .#homeConfigurations.me         # activationPackage
$ nix-why-build .#darwinConfigurations.mac      # system
$ nix-why-build nixpkgs#hello                   # as is
$ nix-why-build /nix/store/...-foo.drv          # a .drv you already have
```

Caches and keys come from `nix config show`, `extra-substituters` included.
A narinfo only counts when a trusted key signed it, like nix itself. Flags
override both.

```console
$ nix-why-build .#nixosConfigurations.tank \
    --substituter https://cache.nixos.org \
    --trusted-key cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
```

It's quick. On tank (a desktop, five caches) the whole report takes under
40 s, 17 of them nix's own evaluation of the system. The option index
evaluates while the caches are asked. `RUST_LOG=info` prints walk timings.

`--json` prints everything, each cascade with its `chain` down to the root.

```json
{
  "name": "llama-cpp-0.5.0",
  "reason": {
    "category": "overridden",
    "against": "unstable#pkgsCuda.llama-cpp",
    "against_cached": true,
    "diff": { "added": ["postPatch"], "changed": ["buildInputs", "cmakeFlags"], "inputs_added": ["blas-3"] }
  },
  "requested_by": [
    { "option": "environment.systemPackages", "via": "system-path", "files": ["apps/llama-cpp-16gb.nix"] }
  ],
  "cascades": [{ "name": "system-path", "chain": ["system-path", "llama-cpp-0.5.0"] }]
}
```

That's one root, trimmed. The full report also has `toplevel`,
`substituters`, `to_build`, `unknown` and `warnings`.

## As a library

```rust
let options = nix_why_build::Options::new(".#nixosConfigurations.tank");
let report = nix_why_build::explain(&options).await?;
for root in &report.roots {
    println!("{}: {} {}", root.name, root.reason.category(), root.reason.detail());
}
```

## Install

```bash
nix run github:mlavrinenko/nix-why-build -- .#nixosConfigurations.tank
cargo install nix-why-build
```

Every release also lands on crates.io and as a binary on the
[releases page](https://github.com/mlavrinenko/nix-why-build/releases/latest).
Before the first one, take `main` with
`cargo install --git https://github.com/mlavrinenko/nix-why-build`.

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
