use std::collections::HashMap;

use nix_derivation::StorePath;

use crate::counterparts::Candidate;
use crate::drv::{Drv, read};
use crate::fixture_tests::{DrvSpec, FakeStore, spec};

use super::{Facts, Lookup, Reason, classify};

fn drv(store: &FakeStore, spec: &DrvSpec) -> Drv {
    read(store.dir(), &store.add(spec)).expect("readable drv")
}

fn llama() -> DrvSpec {
    spec("llama-cpp-0.5.0")
        .env("pname", "llama-cpp")
        .env("version", "0.5.0")
}

fn candidate(drv: &StorePath, input: &str, variant: &str, version: &str) -> Candidate {
    Candidate {
        input: input.to_owned(),
        attr: "llama-cpp".to_owned(),
        variant: variant.to_owned(),
        drv: drv.to_absolute_path(),
        version: Some(version.to_owned()),
        ..Candidate::default()
    }
}

fn found<'a>(candidates: &'a [Candidate], tried: &'a [String]) -> Lookup<'a> {
    Lookup::Found(candidates, tried)
}

#[test]
fn a_fetch_names_its_urls_from_env_or_structured_attrs() {
    let store = FakeStore::new();
    let plain = drv(
        &store,
        &spec("src.tar.gz")
            .fixed()
            .env("urls", "https://a/x https://b/x"),
    );
    let single = drv(
        &store,
        &spec("one.tar.gz").fixed().env("url", "https://c/one"),
    );
    let structured = drv(
        &store,
        &spec("s.tar.gz")
            .fixed()
            .env("__json", r#"{"urls":["https://d/s"]}"#),
    );
    let bare = drv(&store, &spec("bare.tar.gz").fixed());
    let facts = Facts::default();
    let urls = |root: &Drv| match classify(store.dir(), root, Lookup::Skipped("n/a"), &facts) {
        Reason::Fetch { urls } => urls,
        other => panic!("not a fetch: {other:?}"),
    };
    assert_eq!(urls(&plain), ["https://a/x", "https://b/x"]);
    assert_eq!(urls(&single), ["https://c/one"]);
    assert_eq!(urls(&structured), ["https://d/s"]);
    assert!(urls(&bare).is_empty());
}

#[test]
fn without_a_pname_or_a_lookup_it_says_unknown_and_why() {
    let store = FakeStore::new();
    let glue = drv(&store, &spec("policies.json"));
    let root = drv(&store, &llama());
    let facts = Facts::default();
    assert_eq!(
        classify(store.dir(), &glue, found(&[], &[]), &facts),
        Reason::Unknown {
            because: "it has no pname to look up in nixpkgs".to_owned()
        }
    );
    assert_eq!(
        classify(
            store.dir(),
            &root,
            Lookup::Skipped("no nixpkgs input"),
            &facts
        ),
        Reason::Unknown {
            because: "no nixpkgs input".to_owned()
        }
    );
}

#[test]
fn a_local_only_root_is_local_only_before_anything_else() {
    let store = FakeStore::new();
    let root = drv(&store, &llama().env("allowSubstitutes", ""));
    assert_eq!(
        classify(store.dir(), &root, found(&[], &[]), &Facts::default()),
        Reason::LocalOnly
    );
}

#[test]
fn no_candidate_at_all_is_no_counterpart_and_lists_what_was_tried() {
    let store = FakeStore::new();
    let root = drv(&store, &llama());
    let tried = vec![
        "nixpkgs#kache".to_owned(),
        "nixpkgs#python3Packages.kache".to_owned(),
    ];
    assert_eq!(
        classify(store.dir(), &root, found(&[], &tried), &Facts::default()),
        Reason::NoCounterpart { tried }
    );
}

/// The root itself, as nixpkgs evaluates it, with `edit` applied.
fn identical(edit: impl Fn(&mut Candidate), facts: &Facts) -> Reason {
    let store = FakeStore::new();
    let root = drv(&store, &llama());
    let mut same = candidate(&root.path, "nixpkgs", "", "0.5.0");
    edit(&mut same);
    classify(store.dir(), &root, found(&[same], &[]), facts)
}

#[test]
fn the_very_same_derivation_is_explained_by_its_meta() {
    let facts = Facts::default();
    let against = "nixpkgs#llama-cpp".to_owned();
    assert_eq!(
        identical(|c| c.unfree = true, &facts),
        Reason::Unfree {
            against: against.clone()
        }
    );
    assert_eq!(
        identical(|c| c.broken = true, &facts),
        Reason::NotBuiltByHydra {
            against: against.clone(),
            because: "meta.broken".to_owned()
        }
    );
    assert_eq!(
        identical(|c| c.hydra_platforms = Some(vec![]), &facts),
        Reason::NotBuiltByHydra {
            against: against.clone(),
            because: "meta.hydraPlatforms = [ ]".to_owned()
        }
    );
    assert_eq!(
        identical(
            |c| c.hydra_platforms = Some(vec!["aarch64-linux".to_owned()]),
            &facts
        ),
        Reason::NotBuiltByHydra {
            against,
            because: "meta.hydraPlatforms = [ aarch64-linux ]".to_owned()
        }
    );
}

#[test]
fn the_very_same_derivation_on_a_revision_the_cache_lacks_is_newer_than_the_cache() {
    let hello = |served: bool| Facts {
        served: HashMap::from([("/nix/store/h-hello.drv".to_owned(), served)]),
        hello: HashMap::from([("nixpkgs".to_owned(), "/nix/store/h-hello.drv".to_owned())]),
    };
    let on_hydra = |c: &mut Candidate| c.hydra_platforms = Some(vec!["x86_64-linux".to_owned()]);
    assert_eq!(
        identical(on_hydra, &hello(false)),
        Reason::NewerThanCache {
            against: "nixpkgs#llama-cpp".to_owned()
        }
    );
    assert_eq!(
        identical(on_hydra, &hello(true)),
        Reason::Unknown {
            because: "nixpkgs#llama-cpp builds exactly this, yet no cache has it; Hydra may have failed it"
                .to_owned()
        }
    );
    assert!(matches!(
        identical(|_| {}, &Facts::default()),
        Reason::Unknown { .. }
    ));
}

#[test]
fn an_override_diffs_against_a_cached_counterpart_first_then_the_closest() {
    let store = FakeStore::new();
    let root = drv(
        &store,
        &llama()
            .env("postPatch", "patch")
            .env("cmakeFlags", "-DBLAS=TRUE"),
    );
    let vanilla = store.add(
        &llama()
            .env("cmakeFlags", "-DCUDA=FALSE")
            .env("doCheck", "1"),
    );
    let cuda = store.add(&llama().env("cmakeFlags", "-DBLAS=FALSE"));
    let near_but_uncached = store.add(&llama().env("cmakeFlags", "-DBLAS=TRUE"));
    let older = store.add(
        &spec("llama-cpp-0.4.0")
            .env("pname", "llama-cpp")
            .env("version", "0.4.0"),
    );
    let candidates = [
        candidate(&older, "nixpkgs", "", "0.4.0"),
        candidate(&near_but_uncached, "other", "", "0.5.0"),
        candidate(&vanilla, "unstable", "", "0.5.0"),
        candidate(&cuda, "unstable", "pkgsCuda", "0.5.0"),
    ];
    let facts = Facts {
        served: HashMap::from([
            (vanilla.to_absolute_path(), true),
            (cuda.to_absolute_path(), true),
            (near_but_uncached.to_absolute_path(), false),
        ]),
        hello: HashMap::new(),
    };
    let Reason::Overridden {
        against,
        against_cached,
        diff,
    } = classify(store.dir(), &root, found(&candidates, &[]), &facts)
    else {
        panic!("not an override");
    };
    assert_eq!(against, "unstable#pkgsCuda.llama-cpp");
    assert!(against_cached);
    assert_eq!(diff.added, ["postPatch"]);
    assert_eq!(diff.changed, ["cmakeFlags"]);
}

#[test]
fn an_override_against_nothing_cached_still_names_the_closest() {
    let store = FakeStore::new();
    let root = drv(&store, &llama().env("postPatch", "patch"));
    let theirs = store.add(&llama());
    let Reason::Overridden {
        against,
        against_cached,
        diff,
    } = classify(
        store.dir(),
        &root,
        found(&[candidate(&theirs, "nixpkgs", "", "0.5.0")], &[]),
        &Facts::default(),
    )
    else {
        panic!("not an override");
    };
    assert_eq!(against, "nixpkgs#llama-cpp");
    assert!(!against_cached);
    assert_eq!(diff.added, ["postPatch"]);
}

#[test]
fn no_counterpart_of_the_same_version_is_unknown_naming_the_versions_found() {
    let store = FakeStore::new();
    let root = drv(&store, &llama());
    let older = store.add(&spec("llama-cpp-0.4.0"));
    let mut unversioned = candidate(&older, "nixpkgs", "pkgsCuda", "x");
    unversioned.version = None;
    assert_eq!(
        classify(
            store.dir(),
            &root,
            found(&[candidate(&older, "nixpkgs", "", "0.4.0"), unversioned], &[]),
            &Facts::default()
        ),
        Reason::Unknown {
            because: "no counterpart has version 0.5.0: nixpkgs#llama-cpp is 0.4.0, nixpkgs#pkgsCuda.llama-cpp is unversioned"
                .to_owned()
        }
    );
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|&name| name.to_owned()).collect()
}

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
fn a_counterpart_of_the_same_name_counts_when_versions_are_spelled_differently() {
    let store = FakeStore::new();
    let root = drv(
        &store,
        &spec("glibc-locales-2.42-84")
            .env("pname", "glibc-locales")
            .env("version", "2.42-84")
            .env("preBuild", "ours"),
    );
    let theirs = store.add(
        &spec("glibc-locales-2.42-84")
            .env("pname", "glibc-locales")
            .env("preBuild", "theirs"),
    );
    let mut glibc = candidate(&theirs, "nixpkgs", "", "2.42");
    glibc.attr = "glibcLocales".to_owned();
    let Reason::Overridden { against, diff, .. } =
        classify(store.dir(), &root, found(&[glibc], &[]), &Facts::default())
    else {
        panic!("not an override");
    };
    assert_eq!(against, "nixpkgs#glibcLocales");
    assert_eq!(diff.changed, ["preBuild"]);
}

#[test]
fn a_structured_single_url_is_a_fetch_url_too() {
    let store = FakeStore::new();
    let root = drv(
        &store,
        &spec("one.tar.gz")
            .fixed()
            .env("__json", r#"{"url":"https://e/one"}"#),
    );
    assert_eq!(
        classify(
            store.dir(),
            &root,
            Lookup::Skipped("n/a"),
            &Facts::default()
        ),
        Reason::Fetch {
            urls: vec!["https://e/one".to_owned()]
        }
    );
}

#[test]
fn drv_names_drop_the_hash_and_the_suffix() {
    assert_eq!(
        super::drv_name("/nix/store/abc-glibc-locales-2.42-84.drv"),
        "glibc-locales-2.42-84"
    );
    assert_eq!(super::drv_name("plain"), "plain");
}
