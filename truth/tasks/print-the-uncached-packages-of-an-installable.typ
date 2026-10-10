#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Print the uncached packages of an installable",
  kind: "feat",
  priority: ice(
    impact: 9,
    confidence: 0.8,
    ease: 6,
  ),
  difficulty: 4,
  tags: ("cli", "eval"),
  estimate: (src-files: 5),
  x-goals: (
    product: (
      store-independent: helps[the user-facing answer is the walk's, not `nix build --dry-run`'s],
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: helps[one `nix eval` plus the parallel walk],
      embeddable: helps[`explain()` returns a serializable report; the binary only prints it],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: helps[the README shows real output],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
  links: parent(
    "list-uncached-packages-for-a-flake-configuration-store-independent.typ",
  ),
)

== Problem

+ Run `nix-why-build .#nixosConfigurations.tank`.
+ 🔴 It prints `Hello from .#nixosConfigurations.tank!`: the template's greeting.


== TO-BE

- 🟢 `nix-why-build .#nixosConfigurations.tank` evaluates the toplevel, walks it, and prints each uncached package, one per line, with a count of hidden config glue.
- 🟢 `.#homeConfigurations.<n>` and `.#darwinConfigurations.<n>` resolve to their activation package and system; any other installable, or a `.drv` path, is taken as is.
- 🟢 `--substituter` and `--trusted-key` replace what `nix config show` says; `--json` prints the report.


== Changes

- `tests/cli.rs`: red tests first, over a fake store dir, a `file://` cache and a fake `nix` on `PATH`.
- `src/config.rs`: parse `nix config show --json`.
- `src/eval.rs`: map an installable to its toplevel attribute and evaluate its `drvPath`.
- `src/lib.rs`: `explain()` and the `Report` type.
- `src/main.rs`: flags, human and JSON printing.


== Alternatives

- `nix path-info --derivation`: same cost, but it also accepts store paths of outputs, which have no `.drv` to walk.


== Out of scope

- Grouping by root. Milestone 2.
- The flake's own `nixConfig` substituters. Nix only uses them with `accept-flake-config`.


== Dogfood

Run it on `~/.nix` `.#nixosConfigurations.tank`, time it, and paste the output into the README.

