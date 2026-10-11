#import "@local/mindtape:0.2.0": *

#show: task.with(
  title: "Thank the crates it stands on",
  status: done(2026, 10, 11),
  tags: ("tweak",),
  estimate: (src-files: 1),
)

The README named none of the crates that parse derivations, verify narinfos and talk to the caches
