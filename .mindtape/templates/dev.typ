#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: slot(
    "title",
    prompt: "Task title, imperative verb first",
    example: "Cover the 5xx narinfo path in the test suite",
  ),
  kind: "dev",
  priority: ice(
    impact: slot("impact", kind: "int", default: 5),
    confidence: slot("confidence", kind: "float", default: 0.8),
    ease: slot("ease", kind: "int", default: 5),
  ),
  difficulty: slot("difficulty", kind: "int", default: 3),
  tags: slot("tags", kind: "list", default: ()),
  estimate: slot("estimate", kind: "text", default: "(src-files: 2)"),
  x-goals: (
    product: (
      store-independent: slot("store-independent", kind: "raw", default: "neutral"),
      roots-first: slot(
        "roots-first",
        kind: "raw",
        default: "neutral",
        example: "helps[a cache outage no longer floods the roots]",
      ),
      names-requester: slot("names-requester", kind: "raw", default: "neutral"),
      names-reason: slot("names-reason", kind: "raw", default: "neutral"),
      fast: slot("fast", kind: "raw", default: "neutral"),
      embeddable: slot("embeddable", kind: "raw", default: "neutral"),
    ),
    process: (
      sound-codebase: slot("sound-codebase", kind: "raw", default: "neutral"),
      docs-match-code: slot("docs-match-code", kind: "raw", default: "neutral"),
    ),
  ),
)

== Problem

#slot(
  "problem",
  kind: "raw",
  prompt: "Numbered steps anyone can follow, commands in backticks; open the step that shows the defect with 🔴 and quote what it printed",
  example: ```typ
  + Run `nix-why-build .#nixosConfigurations.mypc` while one substituter answers 503.
  + 🔴 Every path that substituter serves prints as a root miss.
  ```,
)

== TO-BE

#slot(
  "to-be",
  kind: "raw",
  prompt: "Bullets opening with 🟢: how it should work, behave and feel once this lands",
  example: ```typ
  - 🟢 A 5xx marks the path unknown, and the report names the failing substituter once.
  ```,
)

== Changes

#slot(
  "changes",
  kind: "raw",
  prompt: "One bullet per file: `path`, then what changes there",
  example: ```typ
  - `src/cache_tests.rs`: the red test first, a 503 narinfo reading as unknown.
  - `src/cache.rs`: map a 5xx to unknown instead of a miss.
  ```,
)

== Rollback

#slot(
  "rollback",
  kind: "raw",
  prompt: "How to undo this if it breaks something: what to revert or restore",
  example: ```typ
  - Revert the commit. No output format or flag changes.
  ```,
)

== Out of scope

#slot(
  "out-of-scope",
  kind: "raw",
  prompt: "Bullets: what not to touch, and why",
  example: ```typ
  - Retries. A flaky cache gets its own task once someone hits it.
  ```,
)

== Dogfood

#slot(
  "dogfood",
  kind: "raw",
  prompt: "How this repo will use the change, or Not applicable and why",
  example: ```typ
  The dogfood run stays readable while a cachix cache hiccups.
  ```,
)
