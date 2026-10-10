#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Fix the cargo-mutants config key cargo-mutants 27 refuses",
  status: done(2026, 10, 10),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

cargo-mutants 27 rejects `exclude` with "unknown field", so no mutants recipe ran at all
