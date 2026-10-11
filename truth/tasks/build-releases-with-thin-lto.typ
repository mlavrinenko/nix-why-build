#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Build releases with thin LTO",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

Fat LTO with one codegen unit kept one core busy for 69 s per release build to save 1.2 MB
