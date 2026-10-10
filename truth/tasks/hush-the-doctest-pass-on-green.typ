#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Hush the doctest pass on green",
  status: done(2026, 10, 10),
  tags: ("tweak",),
  assignee: "agent",
  estimate: (src-files: 1),
)

A green just test printed six doctest lines after nextest's one-line summary, on every gate run; the pass now prints only when it fails.
