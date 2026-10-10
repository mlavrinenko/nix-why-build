#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Generate the JSON schema of the report",
  kind: "feat",
  priority: ice(
    impact: 5,
    confidence: 0.8,
    ease: 7,
  ),
  difficulty: 2,
  tags: ("report",),
  estimate: (src-files: 5),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[a caller in any language reads `--json` against a schema, not against the README],
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: helps[the schema is generated from the types and a test fails when it lags],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 11),
)

== Problem

+ Look for a description of the `--json` output.
+ 🔴 The README shows one hand-trimmed root and says the full report also has `toplevel`, `substituters`, `to_build`, `unknown` and `warnings`; the shape of each, and every `reason` variant, is nowhere.


== TO-BE

- 🟢 `docs/report.schema.json` is a JSON Schema of `--json`, with every field described by its rustdoc.
- 🟢 It is generated from the types: `just schema` rewrites it, and `just test` fails while it lags.


== Changes

- `Cargo.toml`: `schemars` as a dev-dependency, so the shipped crate gains nothing.
- `src/report.rs`, `src/reason.rs`, `src/diff.rs`: derive `JsonSchema` under `cfg(test)`.
- `src/report_tests.rs`: the red test, comparing the generated schema with the committed one.
- `docs/report.schema.json`: the generated schema.
- `Justfile`: `just schema`.
- `.mmz/config.yaml`: the schema joins the test inputs.


== Alternatives

- A `--json-schema` flag. It grows the CLI for a file the repo can carry, and the README needs a link either way.
- schemars as a regular dependency. Every user would compile it for a file only this repo writes.
- A hand-written schema. It lags the types the first time nobody remembers it.


== Out of scope

- The shape of `--json`. The schema describes it as it is.


== Dogfood

The README links the schema for `--json`.

