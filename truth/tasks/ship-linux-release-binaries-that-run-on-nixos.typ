#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Ship Linux release binaries that run on NixOS",
  kind: "fix",
  priority: ice(
    impact: 6,
    confidence: 0.8,
    ease: 5,
  ),
  difficulty: 4,
  tags: ("tooling",),
  estimate: (src-files: 3),
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
      sound-codebase: helps[the Linux binaries come from the flake, so a local `nix build` reproduces the release],
      docs-match-code: helps[the README's "binary on the releases page" works where the tool is used],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Read `.github/workflows/release.yml`: Linux binaries are `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, built with rustup on Ubuntu, and there is an `x86_64-pc-windows-msvc` one.
+ 🔴 A glibc-linked binary from Ubuntu does not start on NixOS, the tool's main audience: there is no `/lib64/ld-linux-x86-64.so.2`, only a stub that refuses it.
+ 🔴 Nix does not run on Windows, so the Windows binary can do nothing but fail, and its build costs a runner per release.


== Environment

- `main` at `57f2260`; the workflow has not run a release yet.


== TO-BE

- 🟢 Linux release binaries are static musl builds from the flake (`packages.static`), for x86_64 and aarch64, so they run on NixOS and on any other distro with nix.
- 🟢 `nix build .#static` reproduces the release binary locally; `file` calls it statically linked and it runs `--version`.
- 🟢 The macOS binaries stay; the Windows one is gone, from `release.yml` and from the `deny.toml` targets.


== Changes

- `flake.nix`: `packages.static`, built with `pkgsStatic`. If `aws-lc-sys` will not build static, switch reqwest's rustls provider to `ring` rather than fight cmake.
- `.github/workflows/release.yml`: Linux jobs run `nix build .#static` on `ubuntu-latest` and `ubuntu-24.04-arm`; drop Windows.
- `deny.toml`: musl targets in, Windows out.


== Out of scope

- A static macOS build: Apple does not support one.


== Dogfood

`nix build .#static` in this repo gives a binary that runs on tank, a NixOS box.

