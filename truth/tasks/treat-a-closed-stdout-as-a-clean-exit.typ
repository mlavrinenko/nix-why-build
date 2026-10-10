#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Treat a closed stdout as a clean exit",
  kind: "fix",
  priority: ice(
    impact: 4,
    confidence: 0.9,
    ease: 8,
  ),
  difficulty: 2,
  tags: ("cli",),
  estimate: (src-files: 1),
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
      sound-codebase: helps[a CLI test pins how the binary behaves in a pipeline],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Run `nix-why-build --json <tank toplevel .drv> --substituter https://cache.nixos.org --trusted-key cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= | head -c 100`.
+ 🔴 It printed `Error: Broken pipe (os error 32)` and exited 1. `src/main.rs` turns the `EPIPE` of a closed stdout into an error, so `| head`, a `less` quit early and `| jq ... | head` all end in a scary line.


== Environment

- `nix-why-build` built from `main` at `57f2260`, Nix 2.34.8, on NixOS x86_64-linux.


== TO-BE

- 🟢 A closed stdout ends the run quietly with exit 0, as `ripgrep` does.
- 🟢 Any other write error still fails loudly.


== Changes

- `tests/cli.rs`: red test first. Spawn the binary with a piped stdout, drop the read end right after spawning, and assert exit 0 with no `Broken pipe` on stderr.
- `src/main.rs`: map an `io::ErrorKind::BrokenPipe` from writing the report to a clean exit.


== Out of scope

- Restoring the default `SIGPIPE` handler: it needs `unsafe` through `libc` for the same result.


== Dogfood

`nix-why-build --json .#nixosConfigurations.tank | jq ... | head` in `~/.nix` stays quiet.

