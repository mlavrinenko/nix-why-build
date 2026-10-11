#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Date the changelog for 0.1.0",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

just release refuses a version with no dated CHANGELOG section
