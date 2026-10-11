# Architecture

nix-why-build answers the way nix would on an empty store, so what you built
yesterday doesn't change the answer.

1. **Eval** (`eval.rs`). nix turns the installable into the `.drv` it builds.
   A NixOS config means its `config.system.build.toplevel`, home-manager its
   `activationPackage`, nix-darwin its `system`.
2. **Walk** (`walk.rs`, `drv.rs`, `cache.rs`). Go down the `.drv` graph from
   there and ask every cache for each output. Stop where one serves it, the
   same cut nix makes. `allowSubstitutes = false` gets built anyway, like nix
   does.
3. **Split** (`roots.rs`). A root is built while everything it needs is
   cached. The rest is dragged in, built only because a root below it is.
4. **Wanted by** (`requester.rs`, `modules.rs`, `index.nix`). An index of the
   config's options, evaluated during the walk, names the options and module
   files above each root.
5. **Why** (`lookup.rs`, `counterparts.rs`, `counterparts.nix`, `reason.rs`,
   `diff.rs`). Fetches, local-only builds and `cargoLock` crates are told
   from the `.drv` alone; every other root is diffed with the same attribute
   in every nixpkgs input of the flake. No evidence, no category. A pname no guessed attribute
   finds goes through `renamed.rs` and `renamed.nix`, an index of each input
   by pname, built on that miss only and cached per input revision.
6. **Print** (`report.rs`, `human.rs`, `style.rs`). Text for you, `--json`
   for scripts.

Caches and keys come from `nix config show` (`config.rs`), unless
`--substituter` and `--trusted-key` say otherwise. A narinfo counts only when
a trusted key signed it. A cache it can't ask (`s3://`, `ssh-ng://`) or reach
counts as a miss, like in nix.

`just oracle <installable>` checks the walk against `nix build --dry-run`, and
CI runs it on every push.
