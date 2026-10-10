#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: slot(
    "title",
    prompt: "what should change, imperative verb first",
    example: "Silence the cargo machete banner in the machete gate",
  ),
  kind: "dev",
  priority: ice(
    impact: slot(
      "impact",
      kind: "int",
      prompt: "1-10, rises with what staying unfiled costs",
      example: 4,
    ),
    confidence: slot(
      "confidence",
      kind: "float",
      prompt: "0-1, high: the evidence below is quoted, not recalled",
      example: 0.9,
    ),
    ease: slot(
      "ease",
      kind: "int",
      prompt: "1-10, falls with how far the fix reaches",
      example: 7,
    ),
  ),
  difficulty: slot(
    "difficulty",
    kind: "int",
    prompt: "0-9 on the judgment scale",
    example: 1,
  ),
  estimate: slot(
    "estimate",
    kind: "text",
    prompt: "(src-files: N)",
    example: "(src-files: 1)",
  ),
  tags: slot("tags", kind: "list", prompt: "found, plus an area tag", example: (
    "found",
    "tooling",
  )),
  x-goals: (
    product: (
      store-independent: slot("store-independent", kind: "raw", default: "neutral"),
      roots-first: slot("roots-first", kind: "raw", default: "neutral"),
      names-requester: slot("names-requester", kind: "raw", default: "neutral"),
      names-reason: slot("names-reason", kind: "raw", default: "neutral"),
      fast: slot("fast", kind: "raw", default: "neutral"),
      embeddable: slot("embeddable", kind: "raw", default: "neutral"),
    ),
    process: (
      sound-codebase: slot(
        "sound-codebase",
        kind: "raw",
        default: "neutral",
        example: "helps[a quiet gate leaves its failures readable]",
      ),
      docs-match-code: slot("docs-match-code", kind: "raw", default: "neutral"),
    ),
  ),
)

== Problem

+ #slot(
    "context",
    kind: "text",
    prompt: "what you were doing when you noticed this, one line",
    example: "Running `just check` while fixing the sort order",
  )
+ 🔴 #raw(slot("path", prompt: "repo-relative path the finding is about", example: "Justfile")): #slot(
    "observation",
    kind: "text",
    prompt: "what is wrong, stated as one fact",
    example: "the `machete` recipe prints two banner lines on every clean run",
  )

#slot(
  "evidence",
  kind: "raw",
  prompt: "the evidence, quoted verbatim from what it printed — command output, a grep hit, a line count",
  example: ````typ
  ```
  Analyzing dependencies of crates in this directory...
  cargo-machete didn't find any unused dependencies in this directory. Good job!
  ```
  ````,
)

This fell outside the task that surfaced it, so it is filed here rather than
left in a closing report no one rereads. It belongs in
#raw(slot("carrier", prompt: "the file that would carry the fix", example: "Justfile")).

== TO-BE

#slot(
  "to-be",
  kind: "raw",
  prompt: "Bullets opening with 🟢: how it should work, behave and feel once this lands",
  example: ```typ
  - 🟢 A clean `just machete` prints nothing, and a dirty one prints the unused crates.
  ```,
)

== Changes

#slot(
  "changes",
  kind: "raw",
  prompt: "One bullet per file: `path`, then what changes there",
  example: ```typ
  - `Justfile`: send the banner to `/dev/null` and keep the exit code.
  ```,
)

== Rollback

#slot(
  "rollback",
  kind: "raw",
  prompt: "How to undo this if it breaks something: what to revert or restore",
  example: ```typ
  - Revert the `Justfile` line.
  ```,
)

== Out of scope

#slot(
  "out-of-scope",
  kind: "raw",
  prompt: "Bullets: what not to touch, and why",
  example: ```typ
  - The other gates' output. Each noisy gate gets its own finding.
  ```,
)

== Dogfood

#slot(
  "dogfood",
  kind: "raw",
  prompt: "How this repo will use the change, or Not applicable and why",
  example: ```typ
  Every `just check` in this repo prints one line fewer per clean run.
  ```,
)
