#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Print a reason as its category and evidence",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

A library caller printing a root's reason had to spell out category() and detail() every time
