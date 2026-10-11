#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Skip check-tasks on CI",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

CI's dev shell has no MindTape, so the arm ran GNU tape mt and failed the first CI run
