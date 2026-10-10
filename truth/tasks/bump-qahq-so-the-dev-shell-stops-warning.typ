#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Bump qahq so the dev shell stops warning",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.9,
    ease: 8,
  ),
  difficulty: 3,
  tags: ("tooling",),
  estimate: (src-files: 2),
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
      sound-codebase: helps[a dev-shell load prints nothing, and a broken linecop reds `just eject`],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ `nix develop --no-eval-cache -c true`
+ 🔴 It prints `evaluation warning: stdenv.isDarwin is deprecated` and the same for `stdenv.isLinux`: qahq a10960e (0.4.0) evaluates jscpd with them.
+ `echo "limits: [" > .linecop.yaml; just eject`
+ 🔴 It exits 0: the recipe masks every linecop exit, and linecop 0.5+ exits 2 on a broken run.


== TO-BE

- 🟢 The dev shell loads without a warning, on qahq main (ae800a0).
- 🟢 `just eject` masks only linecop's finding exit (1), so a broken run fails it.


== Changes

- `flake.lock`: qahq to ae800a0.
- `Justfile`: `eject` masks only exit 1, as cratemplate 267ee43 does.


== Rollback

- Revert the commit; the old lock comes back with its warnings.


== Out of scope

- The other flake inputs: only qahq moves.


== Dogfood

Not applicable: the dev shell is the product here.

