#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Pin the nix-compat version so cargo-deny sees no wildcard",
  status: done(2026, 10, 10),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

A git dependency without a version is a wildcard, which deny.toml bans
