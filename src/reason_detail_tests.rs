use super::Reason;
use super::tests::names;

fn check(cases: Vec<(Reason, &str, &str)>) {
    for (reason, category, detail) in cases {
        assert_eq!(reason.category(), category);
        assert_eq!(reason.detail(), detail);
    }
}

#[test]
fn override_details_name_what_differs() {
    let overridden = |cached: bool, diff: crate::diff::Diff| Reason::Overridden {
        against: "nixpkgs#x".to_owned(),
        against_cached: cached,
        diff,
    };
    let big = crate::diff::Diff {
        added: names(&["postPatch"]),
        changed: names(&["a", "b", "c", "d"]),
        inputs_added: names(&["blas-3"]),
        inputs_changed: names(&["zlib-1.3", "cmake-4"]),
        ..crate::diff::Diff::default()
    };
    let small = crate::diff::Diff {
        removed: names(&["doCheck"]),
        inputs_removed: names(&["a", "b"]),
        ..crate::diff::Diff::default()
    };
    check(vec![
        (
            overridden(true, big),
            "overridden",
            "vs nixpkgs#x (cached): postPatch added; a, b, c +1 changed; input blas-3 added; inputs zlib-1.3, cmake-4 built differently",
        ),
        (
            overridden(false, small),
            "overridden",
            "vs nixpkgs#x (not cached either): doCheck removed; inputs a, b removed",
        ),
        (
            overridden(true, crate::diff::Diff::default()),
            "overridden",
            "vs nixpkgs#x (cached): no attribute differs",
        ),
    ]);
}

#[test]
fn other_details_read_as_one_line_of_evidence() {
    let x = || "nixpkgs#x".to_owned();
    check(vec![
        (
            Reason::Fetch {
                urls: names(&["https://x/a.exe", "https://y/a.exe"]),
            },
            "fetch",
            "no cache has https://x/a.exe",
        ),
        (
            Reason::Fetch { urls: vec![] },
            "fetch",
            "no cache has the source",
        ),
        (
            Reason::Unfree { against: x() },
            "unfree",
            "Hydra skips unfree nixpkgs#x",
        ),
        (
            Reason::NotBuiltByHydra {
                against: x(),
                because: "meta.broken".to_owned(),
            },
            "not-built-by-hydra",
            "nixpkgs#x has meta.broken",
        ),
        (
            Reason::NewerThanCache { against: x() },
            "newer-than-cache",
            "nixpkgs#x is unchanged, but that nixpkgs' hello is not cached either",
        ),
        (
            Reason::NoCounterpart {
                tried: names(&["a#k", "b#k", "a#K", "b#K"]),
            },
            "no-counterpart",
            "no nixpkgs attribute matches its pname (tried a#k, b#k, a#K +1)",
        ),
        (
            Reason::Vendored,
            "vendored",
            "cargoLock.lockFile builds each crate in its own derivation; cargoHash fetches them all in one",
        ),
        (
            Reason::LocalOnly,
            "local-only",
            "allowSubstitutes = false: nix builds it even when a cache has it",
        ),
        (
            Reason::Unknown {
                because: "why not".to_owned(),
            },
            "unknown",
            "why not",
        ),
    ]);
}

#[test]
fn a_reason_prints_its_category_then_its_evidence() {
    let reason = Reason::Unfree {
        against: "nixpkgs#steam".to_owned(),
    };
    assert_eq!(
        reason.to_string(),
        "unfree: Hydra skips unfree nixpkgs#steam"
    );
}
