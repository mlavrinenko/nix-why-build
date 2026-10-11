#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Publish to crates.io through Trusted Publishing",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

A stored registry token lives forever in the repo's secrets; an OIDC one lives for one job
