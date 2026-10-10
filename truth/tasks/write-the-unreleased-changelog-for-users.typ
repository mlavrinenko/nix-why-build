#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Write the unreleased changelog for users",
  status: done(2026, 10, 10),
  tags: ("tweak",),
  assignee: "agent",
  estimate: (src-files: 1),
)

The Unreleased section read as a dev log of internal modules, five of them private since the API narrowed, with features filed under Changed for a crate that never shipped.
