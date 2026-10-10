#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Count the derivations nix builds locally by design",
  kind: "dev",
  priority: ice(
    impact: 6,
    confidence: 0.95,
    ease: 7,
  ),
  difficulty: 3,
  estimate: (src-files: 3),
  tags: ("found", "walk"),
  x-goals: (
    product: (
      store-independent: helps[the report matches nix's own plan on an empty store, derivation for derivation],
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[a local-only build gets its own reason, allowSubstitutes = false, instead of passing as served],
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: helps[a just recipe pins the tool to nix's own planner, so a walk regression shows as a diff],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: done(2026, 10, 10),
)

== Problem

+ Cross-checking \`to\_build\` against \`nix build --dry-run\` on an empty store that holds only tank's derivations
+ 🔴 #raw("src/walk.rs"): the walk counts a served derivation as fetched, but nix builds every derivation with \`allowSubstitutes = false\` locally even when a cache serves it

```
$ nix copy --no-check-sigs --to "local?root=$s" $top
$ nix build --dry-run --store "local?root=$s" "$top^*"   # 1673 will be built
$ nix-why-build --json .#nixosConfigurations.tank        # 1469 to_build
only nwb: 0
only nix: 204   (184 allowSubstitutes = "", 20 structuredAttrs false)
```
e.g. `nodejs-24.21.0` (a preferLocalBuild wrapper), `etc-bash_logout`, `unit-sockets.target`, all served by cache.nixos.org.


This fell outside the task that surfaced it, so it is filed here rather than
left in a closing report no one rereads. It belongs in
#raw("src/walk.rs").

== TO-BE

- 🟢 A derivation with `allowSubstitutes = false` (env `""` or structured `false`) counts as built, with its own reason, and the walk descends into its inputs as nix does.
- 🟢 `to_build` equals nix's dry-run "will be built" set on an empty store for tank.
- 🟢 `just oracle <installable>` copies the derivation closure into a scratch store, runs nix's dry-run there, and diffs it with `to_build`; empty output means they agree.


== Changes

- `src/walk.rs`: treat `allowSubstitutes = false` as unserved without querying caches.
- `src/reason.rs` (or wherever reasons live): a `local-only` reason.
- `Justfile`: the `oracle` recipe.


== Rollback

- Revert the commit; the walk returns to cache-only answers.


== Out of scope

- Hiding these from the default human report: they are pname-less glue mostly, which the report already folds into the "without a pname" count.


== Dogfood

Run `just oracle .#nixosConfigurations.tank` from ~/.nix after the change; it must print nothing.
