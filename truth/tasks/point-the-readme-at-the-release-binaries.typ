#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Point the README at the release binaries",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

Readers without cargo or flakes had no install path
