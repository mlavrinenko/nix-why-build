#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Fork a worktree with one recipe",
  status: done(2026, 10, 10),
  tags: ("tweak",),
  assignee: "agent",
  estimate: (src-files: 1),
)

Every parallel agent needed a hand-made worktree plus direnv allow, and install-hooks failed inside a worktree, whose .git is a file.
