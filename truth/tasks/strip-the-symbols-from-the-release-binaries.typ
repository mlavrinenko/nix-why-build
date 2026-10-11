#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Strip the symbols from the release binaries",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

nixpkgs turns Cargo's strip = true off and stdenv keeps the symbol table, so the binary carried 1 MB of symbols
