#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Resolve every installable nix accepts",
  kind: "fix",
  priority: ice(
    impact: 7,
    confidence: 0.9,
    ease: 7,
  ),
  difficulty: 3,
  tags: ("eval", "cli"),
  estimate: (src-files: 1),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: helps[a quoted configuration name gets its option index, so `wanted by` names files again],
      names-reason: helps[a flake installable without `#` still finds its nixpkgs inputs],
      fast: neutral,
      embeddable: helps[`explain` takes what `nh` hands `nix build`, with no syntax of its own to trip on],
    ),
    process: (
      sound-codebase: helps[nix parses installables, so the CLI stops re-implementing that grammar],
      docs-match-code: helps[the README's "It takes any installable" becomes true],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Run `nix-why-build .` in this repo, whose flake has a `packages.default`.
+ 🔴 It printed `Error: nix eval --raw ..drvPath failed:` and `getting status of ".../..drvPath": No such file or directory`. `src/eval.rs` appends `.drvPath` to the installable as text, so an installable without a fragment (`.`, `github:o/r`, `nixpkgs`) breaks.
+ Run `nix-why-build '.#nixosConfigurations."host.lan"'` on a flake with such a configuration.
+ 🔴 `config_attr` and `toplevel_attr` split the attribute path on every dot, quoted ones included, so the configuration is not recognised: no toplevel suffix, no option index.


== Environment

- `nix-why-build` built from `main` at `57f2260`, Nix 2.34.8, on NixOS x86_64-linux.


== TO-BE

- 🟢 `nix-why-build .` explains this repo's own package, and so does any installable `nix build` accepts. Nix resolves the `.drv` itself (`nix path-info --derivation <installable>`), so the CLI never re-implements installable syntax.
- 🟢 A configuration whose quoted name holds dots resolves to its toplevel and gets its option index.
- 🟢 A flake installable without `#` still gets `why` lines: its flake reference is the whole installable.
- 🟢 A `.drv` path still runs without nix evaluating anything.


== Changes

- `tests/cli.rs`: red tests first. The fake `nix` logs its arguments: `.` reaches it as `path-info --derivation .`, and `.#nixosConfigurations."a.b"` as `path-info --derivation .#nixosConfigurations."a.b".config.system.build.toplevel`.
- `src/eval.rs`: resolve through `nix path-info --derivation`; split attribute paths on dots outside quotes; `flake_ref` returns the whole installable when it has no `#` and is not a store path.


== Out of scope

- Output paths such as `./result`: nix's own error already says what is wrong.


== Dogfood

`nix-why-build .` in this repo prints the roots of its own package build: the crate and its vendored sources.

