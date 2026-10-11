#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Let just release push the first main",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

just release fetched origin main, so a repo that was never pushed could not get its first release
