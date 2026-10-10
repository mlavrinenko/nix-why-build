#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Rewrite the README in the maintainer's voice",
  kind: "docs",
  priority: ice(
    impact: 6,
    confidence: 0.8,
    ease: 7,
  ),
  difficulty: 2,
  tags: ("docs",),
  estimate: (src-files: 5),
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
      sound-codebase: neutral,
      docs-match-code: helps[a README short enough to keep in step with the code],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 11),
)

== Problem

+ Read `README.md`.
+ 🔴 196 lines in nobody's voice. The example runs on `.#nixosConfigurations.tank` and prints `home-manager.users.tank` paths, `It's quick. On tank ...` reads as a benchmark, and a paragraph explains the terminal colors.


== TO-BE

- 🟢 The README says what the tool lists and why, shows a short example, the reasons, the targets, `--json` with its schema and the library, in the maintainer's words.
- 🟢 How it works lives in `docs/architecture.md`; development lives in `CONTRIBUTING.md`.
- 🟢 No user-facing text names the maintainer's machine.


== Source of truth

- `src/reason.rs`: the categories and their evidence.
- `src/eval.rs`: what each kind of installable resolves to.
- A run on a desktop NixOS configuration: the example's lines.


== Changes

- `README.md`: rewritten.
- `docs/architecture.md`: the walk in six steps.
- `src/lib.rs`: the crate example matches the README's and compiles; the README is no longer a doctest.
- `src/main.rs`: `--help` names `.#nixosConfigurations.mypc`.
- `CHANGELOG.md`: no timings or derivation counts from the maintainer's machine.
- `outdatty.yaml`, `.mmz/config.yaml`, `.mindtape/config.toml`: track `docs/`.


== Out of scope

- Test fixtures named after the maintainer's machine. Nobody reads them as docs.


== Dogfood

The README's example comes from running the tool on the maintainer's own configuration.

