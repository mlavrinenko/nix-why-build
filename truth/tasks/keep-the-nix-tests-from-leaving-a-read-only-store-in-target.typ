#import "@local/mindtape:0.2.0": *
#import "../lib.typ": *

#show: task.with(
  title: "Keep the nix tests from leaving a read-only store in target",
  kind: "fix",
  priority: ice(
    impact: 6,
    confidence: 0.9,
    ease: 8,
  ),
  difficulty: 2,
  tags: ("tests", "tooling"),
  estimate: (src-files: 1),
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
      sound-codebase: helps[`cargo clean` and `git worktree remove` work again after a test run],
      docs-match-code: neutral,
    ),
  ),
  assignee: "agent",
  status: proposed(2026, 10, 10),
)

== Problem

+ Run `just test` once, then `cargo clean`.
+ 🔴 It failed with `Permission denied (os error 13)`. The real-nix tests write their derivations into a scratch `local?root=target/nix-scratch` store, whose store paths nix makes read-only, so `cargo clean`, `rm -rf target` and `git worktree remove` all stop there.


== Environment

- `main` at `e770648`, Nix 2.34.8, on NixOS x86_64-linux.


== TO-BE

- 🟢 The real-nix tests run against an in-memory store (`dummy://?read-only=false`): nothing lands on disk, and each `nix` process starts empty, so parallel tests share no state.
- 🟢 The guard in `src/nix_tests.rs` refuses any other store, so a plain `cargo test` still cannot write to `/nix/store`.
- 🟢 `cargo clean` works after `just test`.


== Changes

- `src/nix_tests.rs`: red first, the guard asks for the in-memory store; then its doc comment says why.
- `tests/support/nix-scratch-store.sh`: export `NIX_REMOTE=dummy://?read-only=false` and nothing else.
- `.config/nextest.toml`: the comment follows.


== Out of scope

- Dropping the setup script: tests cannot set the environment of the `nix` they spawn without a hook in product code.


== Dogfood

`just fork` worktrees can be removed after their gates ran.

