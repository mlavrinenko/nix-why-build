#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Run on a nix without flakes enabled",
  kind: "fix",
  priority: ice(
    impact: 4,
    confidence: 0.8,
    ease: 9,
  ),
  difficulty: 1,
  tags: ("eval",),
  estimate: (src-files: 1),
  x-goals: (
    product: (
      store-independent: helps[the answer no longer depends on a local `experimental-features` setting],
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Run `NIX_CONFIG='experimental-features =' nix-why-build nixpkgs#hello`, standing in for a machine that uses flakes only through `nixos-rebuild --flake`, which enables them per call.
+ 🔴 It printed `Error: nix config show --json failed:` and `experimental Nix feature 'nix-command' is disabled`.


== Environment

- `nix-why-build` built from `main` at `57f2260`, Nix 2.34.8, on NixOS x86_64-linux.


== TO-BE

- 🟢 Every `nix` call passes `--extra-experimental-features 'nix-command flakes'`, as `nixos-rebuild` does, so the tool runs wherever `nixos-rebuild --flake` does.


== Changes

- `tests/cli.rs`: red test first. The fake `nix` logs its arguments, and every logged call carries the flag.
- `src/eval.rs`: prepend the flag in `nix()`.


== Out of scope

- Nix older than 2.4, which has neither feature.


== Dogfood

Not applicable: tank enables flakes globally, so the change is invisible here.

