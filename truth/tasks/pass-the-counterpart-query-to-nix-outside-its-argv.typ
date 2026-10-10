#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Pass the counterpart query to nix outside its argv",
  kind: "fix",
  priority: ice(
    impact: 5,
    confidence: 0.7,
    ease: 7,
  ),
  difficulty: 3,
  tags: ("eval",),
  estimate: (src-files: 1),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[a mass rebuild keeps its reasons instead of failing the whole lookup],
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: helps[a unit test pins the argv size, which no gate checks today],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Read `src/counterparts.rs`: `find` passes every lookup inside the one `--expr` argument, as an escaped JSON string.
+ 🔴 Linux refuses any single argument over 128 KiB (`MAX_ARG_STRLEN`) with `E2BIG`. A lookup costs about 160 bytes, so around 800 distinct roots with a pname break the call, and every root says `unknown: looking it up in nixpkgs failed: run nix: Argument list too long`. That is the `newer-than-cache` case itself: right after a nixpkgs bump, before Hydra catches up, most packages are roots.


== Environment

- `nix-why-build` built from `main` at `57f2260`, Nix 2.34.8, on NixOS x86_64-linux.


== TO-BE

- 🟢 The query reaches nix through a file or stdin, so the argv stays the same size whatever the number of roots.
- 🟢 A lookup of 5 000 roots evaluates like one of 5.


== Changes

- `src/counterparts_tests.rs`: red test first. The arguments `find` passes stay under 16 KiB for 5 000 lookups, computed by a pure function so the test needs no nix.
- `src/counterparts.rs`: hand the query JSON over through a temporary file read with `builtins.readFile` (the eval is `--impure` already) or through stdin; the expression itself stays constant.


== Out of scope

- Splitting the lookup into several evaluations: each one imports nixpkgs again.


== Dogfood

Bumping the nixpkgs of `~/.nix` before Hydra finishes still gives every root a `why`.

