#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Give assignee an example",
  status: done(2026, 10, 10),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

The current mt fills every template slot from an example during mt check and refused the assignee slot
