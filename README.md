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
13 fetches: andale32.exe, arial32.exe, arialb32.exe (+10 more)
  wanted by environment.systemPackages (nixpkgs:nixos/modules/programs/steam.nix), environment.etc."lightdm/lightdm.conf" (nixpkgs:nixos/modules/services/x11/display-managers/lightdm.nix) (+1 more)
  drags in corefonts-1, steam-1.0.0.85, steam-run-1.0.0.85
246 fetches: acorn-8.18.0.tgz, aix-ppc64-0.27.7.tgz, aix-ppc64-0.28.2.tgz (+243 more)
  wanted by home-manager.users.tank.xdg.configFile."opencode/plugins/octrimmer.js" (apps/agents/opencode/plugins/octrimmer/octrimmer.nix)
  drags in octrimmer-0.1.1-sources, octrimmer-0.1.1
223 roots: arraydeque-0.5.1, arrayvec-0.7.8, assert_cmd-2.2.2 (+220 more)
  wanted by environment.systemPackages (kache:packaging/nix/module.nix), systemd.user.services.kache (kache:packaging/nix/module.nix) (+4 more)
  drags in kache-1.0.0
russian-trusted-ca-constrained.crt
  wanted by home-manager.users.tank.home.packages (home-manager:modules/programs/firefox), home-manager.users.tank.home.file.".mozilla/firefox/974ct1mu.default/search.json.mozlz4" (home-manager:modules/programs/firefox) (+1 more)
  drags in firefox-156.0.1
...
llama-cpp-0.5.0
  wanted by environment.systemPackages (apps/llama-cpp-16gb.nix), systemd.services.llama-cpp-embed (apps/llama-cpp-16gb.nix, base:apps/gpu-exclusive.nix) (+1 more)
...
514 roots in 29 groups, 45 packages to build, 1424 more without a pname
```

A **root** is something no cache serves even though
everything it needs is served. That's where the miss starts. Everything it
**drags in** only builds because of it. Fix the root, and the rest comes from
the cache again. Above, a custom CA cert is why Firefox compiles, and kache's
223 vendored crates are one line, not 223.

`wanted by` names the options that pull a root in, with the module files that
set them. Your own show as `apps/llama-cpp-16gb.nix`, a flake input's as
`home-manager:modules/...`. That takes one more `nix eval` of the
configuration, run while the caches are asked. When a module only mentions a
package (an `environment.etc` file, a profile script), that's what you get.
It says where the reference is, not why someone wrote it.

Roots that drag in the same packages print as one group. Config glue (units,
`etc`, `system-path`, wrapper scripts) is cheap to build, so it's counted,
not listed. The tally goes to stderr.

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

It's quick. On tank (a desktop with five caches) the whole report takes about
30 s, 17 of them nix's own evaluation of the system. The walk asks the first
cache alone, since it serves most paths, then the rest at once, and resends a
request that stalls.
`RUST_LOG=info` prints the per-level timings.

Want it machine readable? Pass `--json`. Here it is trimmed to one root.

```json
{
  "toplevel": "/nix/store/rlnr...-nixos-system-tank-26.05.20260927.cf5e765.drv",
  "substituters": [
    "https://cache.nixos.org/",
    "https://devenv.cachix.org"
  ],
  "roots": [
    {
      "name": "russian-trusted-ca-constrained.crt",
      "pname": null,
      "version": null,
      "drv": "/nix/store/qnmh...-russian-trusted-ca-constrained.crt.drv",
      "fixed_output": false,
      "cascades": [
        {
          "name": "firefox-156.0.1",
          "pname": "firefox",
          "drv": "/nix/store/a4av...-firefox-156.0.1.drv",
          "chain": [
            "firefox-156.0.1",
            "policies.json",
            "russian-trusted-ca-constrained.crt"
          ]
        }
      ],
      "requested_by": [
        {
          "option": "home-manager.users.tank.home.file.\".mozilla/native-messaging-hosts\"",
          "via": "home-manager-files",
          "via_drv": "/nix/store/r0yc...-home-manager-files.drv",
          "item": "mozilla-native-messaging-hosts",
          "item_drv": "/nix/store/59s9...-mozilla-native-messaging-hosts.drv",
          "files": [
            "home-manager:modules/misc/mozilla-messaging-hosts.nix"
          ]
        }
      ]
    }
  ],
  "to_build": [
    {
      "name": "firefox-156.0.1",
      "pname": "firefox",
      "version": "156.0.1",
      "drv": "/nix/store/a4av...-firefox-156.0.1.drv"
    }
  ],
  "unknown": [],
  "warnings": []
}
```

Every cascade carries its `chain` down to the root, so you see the path. Here
Firefox bakes the cert in through its `policies.json`.

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
