use crate::fixture_tests::{FakeStore, spec};

use super::Diff;

fn diff(
    store: &FakeStore,
    counterpart: &crate::fixture_tests::DrvSpec,
    root: &crate::fixture_tests::DrvSpec,
) -> Diff {
    let theirs = store.add(counterpart);
    let ours = store.add(root);
    Diff::read(store.dir(), &theirs, &ours).expect("both drvs read")
}

fn names(list: &[String]) -> Vec<&str> {
    list.iter().map(String::as_str).collect()
}

#[test]
fn store_hashes_inside_values_do_not_count() {
    let store = FakeStore::new();
    let blas_a = store.add(&spec("blas-3"));
    let blas_b = store.add(&spec("blas-3"));
    let flags = |blas: &nix_derivation::StorePath| {
        format!("-DBLAS={}", store.output(blas, "out").to_absolute_path())
    };
    let diff = diff(
        &store,
        &spec("llama-cpp-0.5.0").env("cmakeFlags", &flags(&blas_a)),
        &spec("llama-cpp-0.5.0").env("cmakeFlags", &flags(&blas_b)),
    );
    assert!(
        diff.changed.is_empty() && diff.added.is_empty() && diff.removed.is_empty(),
        "{diff:?}"
    );
}

#[test]
fn names_added_removed_and_changed_attributes() {
    let store = FakeStore::new();
    let diff = diff(
        &store,
        &spec("llama-cpp-0.5.0")
            .env("cmakeFlags", "-DBLAS=FALSE")
            .env("doCheck", "1"),
        &spec("llama-cpp-0.5.0")
            .env("cmakeFlags", "-DBLAS=TRUE")
            .env("postPatch", "substituteInPlace x"),
    );
    assert_eq!(names(&diff.added), ["postPatch"]);
    assert_eq!(names(&diff.removed), ["doCheck"]);
    assert_eq!(names(&diff.changed), ["cmakeFlags"]);
    assert!(!diff.is_empty());
}

#[test]
fn structured_attrs_compare_key_by_key() {
    let store = FakeStore::new();
    let diff = diff(
        &store,
        &spec("llama-cpp-0.5.0").env(
            "__json",
            r#"{"pname":"llama-cpp","cmakeFlags":["-DBLAS=FALSE"]}"#,
        ),
        &spec("llama-cpp-0.5.0").env(
            "__json",
            r#"{"pname":"llama-cpp","cmakeFlags":["-DBLAS=TRUE"],"postPatch":"x"}"#,
        ),
    );
    assert_eq!(names(&diff.added), ["postPatch"]);
    assert_eq!(names(&diff.changed), ["cmakeFlags"]);
    assert!(diff.removed.is_empty());
}

#[test]
fn inputs_compare_by_name_and_flag_a_same_named_one_that_differs() {
    let store = FakeStore::new();
    let blas = store.add(&spec("blas-3"));
    let gone = store.add(&spec("openblas-0.3"));
    let zlib_theirs = store.add(&spec("zlib-1.3"));
    let zlib_ours = store.add(&spec("zlib-1.3").env("patches", "x"));
    let cmake = store.add(&spec("cmake-4.1"));
    let diff = diff(
        &store,
        &spec("llama-cpp-0.5.0")
            .input(&gone, &["out"])
            .input(&zlib_theirs, &["out"])
            .input(&cmake, &["out"]),
        &spec("llama-cpp-0.5.0")
            .input(&blas, &["out"])
            .input(&zlib_ours, &["out"])
            .input(&cmake, &["out"]),
    );
    assert_eq!(names(&diff.inputs_added), ["blas-3"]);
    assert_eq!(names(&diff.inputs_removed), ["openblas-0.3"]);
    assert_eq!(names(&diff.inputs_changed), ["zlib-1.3"]);
}

#[test]
fn identical_derivations_differ_in_nothing() {
    let store = FakeStore::new();
    let zlib = store.add(&spec("zlib-1.3"));
    let same = spec("hello-2.12")
        .env("pname", "hello")
        .input(&zlib, &["out"]);
    let diff = diff(&store, &same, &same);
    assert!(diff.is_empty(), "{diff:?}");
    assert_eq!(diff.size(), 0);
}

#[test]
fn size_counts_every_difference() {
    let store = FakeStore::new();
    let add = store.add(&spec("blas-3"));
    let gone = [store.add(&spec("gone-1")), store.add(&spec("gone-2"))];
    let theirs = [
        store.add(&spec("z-1")),
        store.add(&spec("y-1")),
        store.add(&spec("x-1")),
    ];
    let ours = [
        store.add(&spec("z-1").env("p", "1")),
        store.add(&spec("y-1").env("p", "1")),
    ];
    let mut counterpart = spec("x").env("a", "1").env("b", "1");
    for input in gone.iter().chain(&theirs) {
        counterpart = counterpart.input(input, &["out"]);
    }
    let mut root = spec("x").env("a", "2").env("c", "1").input(&add, &["out"]);
    for input in &ours {
        root = root.input(input, &["out"]);
    }
    let diff = diff(&store, &counterpart, &root);
    assert_eq!(diff.size(), 9);
}

#[test]
fn an_unreadable_side_is_an_error() {
    let store = FakeStore::new();
    let ours = store.add(&spec("x"));
    let gone = crate::fixture_tests::absent_drv("gone");
    assert!(Diff::read(store.dir(), &gone, &ours).is_err());
    assert!(Diff::read(store.dir(), &ours, &gone).is_err());
}

#[test]
fn blanks_only_real_store_hashes() {
    let hash = "a".repeat(32);
    let cases = [
        (
            format!("/nix/store/{hash}-zlib/lib"),
            "/nix/store/…-zlib/lib".to_owned(),
        ),
        (
            format!("x /nix/store/{hash}-a /nix/store/{hash}-b"),
            "x /nix/store/…-a /nix/store/…-b".to_owned(),
        ),
        (
            "/nix/store/short-zlib".to_owned(),
            "/nix/store/short-zlib".to_owned(),
        ),
        (format!("/nix/store/{hash}"), format!("/nix/store/{hash}")),
        (
            format!("/nix/store/{}-x", "a".repeat(31) + "!"),
            format!("/nix/store/{}-x", "a".repeat(31) + "!"),
        ),
        ("no paths".to_owned(), "no paths".to_owned()),
    ];
    for (input, expected) in cases {
        assert_eq!(super::blank_hashes(&input), expected, "{input}");
    }
}
