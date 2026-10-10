#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Make the flake easy to consume",
  kind: "dev",
  priority: ice(
    impact: 5,
    confidence: 0.9,
    ease: 7,
  ),
  difficulty: 2,
  tags: ("tooling",),
  estimate: (src-files: 2),
  x-goals: (
    product: (
      store-independent: neutral,
      roots-first: neutral,
      names-requester: neutral,
      names-reason: neutral,
      fast: neutral,
      embeddable: helps[a NixOS or home-manager configuration adds the tool through an overlay],
    ),
    process: (
      sound-codebase: helps[CI builds the package `nix run` installs, so it cannot break unnoticed],
      docs-match-code: helps[the README's first install line is checked on every push],
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
)

== Problem

+ Read `flake.nix`: `packages.default` has no `meta`, and there is no `overlays.default`, no `checks` and no `formatter`.
+ 🔴 CI never builds the package: `just check` runs cargo in the dev shell, so a `Cargo.lock` the vendoring rejects or a `src` filter that drops a file breaks `nix run github:mlavrinenko/nix-why-build`, the README's first install line, with CI green.
+ 🔴 Adding the tool to a system means `inputs.nix-why-build.packages.${system}.default` spelled by hand, and `nix flake check` checks nothing.


== TO-BE

- 🟢 `packages.default` carries `meta` (`description`, `homepage`, `license = lib.licenses.mit`, `mainProgram`).
- 🟢 `overlays.default` adds `nix-why-build` to `pkgs`.
- 🟢 `checks.<system>.package` builds it, and CI runs `nix flake check`.
- 🟢 `formatter` is `nixfmt`, so `nix fmt` formats the `.nix` files.
- 🟢 The README's install section shows the overlay in two lines.


== Changes

- `flake.nix`: `meta`, `overlays.default`, `checks`, `formatter`.
- `.github/workflows/ci.yml`: a `nix flake check` step.


== Rollback

- Revert the commit; the flake goes back to a package and a dev shell.


== Out of scope

- A NixOS module: the tool has no service and no configuration to set.


== Dogfood

`~/.nix` can take the tool through `overlays.default` instead of `just install`.

