// What nix-why-build is for. Every open task gives each goal a verdict in its
// `x-goals: (product: ...)` field, checked by `check-goals(...)` from
// truth/lib.typ. How this repo builds it is a separate set, in
// process-goals.typ.

#let goals = (
  store-independent: (
    title: "The answer ignores the local store",
    acid-test: [The report on a machine that already built everything equals
      the report on a fresh machine.],
  ),
  roots-first: (
    title: "Misses group by root",
    acid-test: [Every uncached package names the root miss it cascades from,
      so the user fixes a handful of roots, not a list of fifty packages.],
  ),
  names-requester: (
    title: "Every root names who asked",
    acid-test: [Each root names the NixOS or home-manager option and the
      module file that pulls it in.],
  ),
  names-reason: (
    title: "Every root says why it misses",
    acid-test: [Each root carries a reason category with its evidence (the
      differing attributes against nixpkgs), or says unknown; never a guess.],
  ),
  fast: (
    title: "Faster than the build it predicts",
    acid-test: [A desktop NixOS config reports in under 60 s with a warm eval
      cache.],
  ),
  embeddable: (
    title: "nh can link it",
    acid-test: [A library API that `nh os build --explain` could call without
      shelling out to the binary.],
  ),
)
