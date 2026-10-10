#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Add a just install recipe",
  status: done(2026, 10, 10),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

~/.nix installs local tools through each checkout's just install, and nix-why-build had none
