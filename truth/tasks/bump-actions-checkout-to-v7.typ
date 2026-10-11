#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Bump actions/checkout to v7",
  status: done(2026, 10, 11),
  tags: ("(\"tooling\",)",),
  estimate: (src-files: 1),
)

Dependabot's PR for it stuck on a stale test merge, and v7 breaks only pull\_request\_target and workflow\_run, which we do not use
