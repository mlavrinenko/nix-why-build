#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Find the counterpart whose attribute differs from its pname",
  kind: "dev",
  priority: ice(
    impact: 6,
    confidence: 0.9,
    ease: 4,
  ),
  difficulty: 6,
  estimate: (src-files: 3),
  tags: ("found", "report"),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: helps[a renamed attribute gets a real category instead of no-counterpart],
      fast: neutral,
      embeddable: neutral,
    ),
    process: (
      sound-codebase: neutral,
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 11),
)

== Problem

+ Writing the README example from a run on the maintainer's desktop NixOS config.
+ 🔴 #raw("src/counterparts.rs"): \`attr\_guesses\` derives attributes from the pname alone, so a package whose nixpkgs attribute differs from its pname gets \`no-counterpart\`: \`nss-cacert\` lives at \`nixpkgs\#cacert\`.

```
$ nix-why-build --json ~/.nix#nixosConfigurations.$(hostname) | jq -c '.roots[] | select(.pname == "nss-cacert") | .reason'
{"category":"no-counterpart","tried":["nixpkgs#nss-cacert","unstable#nss-cacert","nixpkgs#nssCacert","unstable#nssCacert","nixpkgs#python3Packages.nss-cacert","unstable#python3Packages.nss-cacert","nixpkgs#perlPackages.Nsscacert","unstable#perlPackages.Nsscacert"]}
$ nix eval --raw nixpkgs#cacert.pname
nss-cacert
```
The root is wanted by `environment.etc."pki/tls/certs/ca-bundle.crt"` (`nixpkgs:nixos/modules/security/ca.nix`), which builds `pkgs.cacert.override { ... }`, so the honest answer is most likely `overridden` against `nixpkgs#cacert`.


This fell outside the task that surfaced it, so it is filed here rather than
left in a closing report no one rereads. It belongs in
#raw("src/counterparts.rs").

== TO-BE

- 🟢 `nss-cacert-3.126` compares against `nixpkgs#cacert` and gets a category with evidence, like `overridden: vs nixpkgs#cacert (cached): ...`.
- 🟢 The common case, a pname that matches its attribute, costs no extra evaluation.


== Changes

Measure first: time `builtins.attrNames` plus `pname or null` under `tryEval` over a whole nixpkgs input, since the goal `fast` caps a desktop config at 60 s.

- `tests/fixtures/`: the fake nixpkgs gains an attribute `cacert` whose pname is `nss-cacert`.
- `src/nix_counterparts_tests.rs`: the red test, a `nss-cacert` lookup that finds `cacert`.
- `src/counterparts.nix`: when every guess misses, a pname-to-attribute index of the input finds it.
- `src/counterparts.rs`, `src/lookup.rs`: build that index only on a miss and cache it under `$XDG_CACHE_HOME` by the input's `narHash`, so the common path pays nothing.


== Rollback

- Revert the commit. Roots go back to `no-counterpart`.


== Out of scope

- Attributes nested deeper than the package sets `attr_guesses` already tries. Wait for a real miss.
- The index on every run. It pays a full top-level evaluation each time.
- `packages.json` from channels.nixos.org. Needs network, and its revision rarely matches the flake's lock.
- A hand table of renames (`cacert`, ...). It rots and misses the next one.


== Dogfood

The maintainer's desktop run prints a category for `nss-cacert` instead of `no-counterpart`.
