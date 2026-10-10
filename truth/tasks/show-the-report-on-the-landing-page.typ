#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Show the report on the landing page",
  kind: "docs",
  priority: ice(
    impact: 3,
    confidence: 0.8,
    ease: 8,
  ),
  difficulty: 2,
  tags: ("docs",),
  estimate: (src-files: 0),
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
      docs-match-code: helps[the page shows the output the tool prints today],
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Open `www/index.html`, the GitHub Pages site.
+ 🔴 It is one sentence and `cargo install nix-why-build`. A Nix user's first command is `nix run`, and the product is its report, which the page never shows.


== TO-BE

- 🟢 The page leads with `nix run github:mlavrinenko/nix-why-build -- .#nixosConfigurations.<host>`, then a trimmed real report, then the `why` table, in the README's voice.
- 🟢 It stays one static HTML file with no build step and no JavaScript.


== Source of truth

- `README.md` holds the canonical example and the `why` table; the page copies a trimmed excerpt, and `outdatty.yaml` already flags it on every `src/` change.


== Changes

- `www/index.html`: install line, example report, `why` table.


== Out of scope

- A docs site generator: one page needs none.


== Dogfood

The report on the page is cut from a real tank run.

