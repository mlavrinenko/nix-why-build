// How this repo builds nix-why-build. Kept apart from goals.typ, which says
// what it is for. Every open task gives each goal here a verdict too, in its
// `x-goals: (process: ...)` field, checked by `check-goals(...)` from
// truth/lib.typ.

#let goals = (
  sound-codebase: (
    title: "Changes stay safe",
    acid-test: [A change that breaks documented behaviour fails `just check`
      before it can be committed, and every file fits its size cap.],
  ),
  docs-match-code: (
    title: "Docs match the code",
    acid-test: [Every doc claim about the CLI, its output or the library API
      is shown by an example that runs, or fails a gate once the two drift.],
  ),
)
