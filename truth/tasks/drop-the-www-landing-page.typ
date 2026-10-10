#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Drop the www landing page",
  kind: "docs",
  priority: ice(
    impact: 3,
    confidence: 0.8,
    ease: 8,
  ),
  difficulty: 1,
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
      docs-match-code: helps[one README to keep in step with the code, not two],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 11),
)

== Problem

+ Diff the prose of `www/index.html` with `README.md`.
+ 🔴 The page is a hand-made copy of the README, so every README edit owes it a second one, and `outdatty` flags it on every `src/` change.


== TO-BE

- 🟢 The README is the only user doc. No `www/`, no Pages workflow.


== Source of truth

- `README.md`.


== Changes

- `www/index.html`: gone.
- `.github/workflows/pages.yml`: gone.
- `.github/workflows/ci.yml`: the paths comment names a docs-only PR.
- `outdatty.yaml`, `.mmz/config.yaml`, `.mindtape/config.toml`: no `www/**`.


== Out of scope

- A site generated from Markdown. Back when the tool outgrows its README.


== Dogfood

Not applicable: it removes a page.

