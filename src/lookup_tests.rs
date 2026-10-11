use crate::counterparts::{Input, Lookup};
use crate::drv::{Drv, read};
use crate::fixture_tests::{DrvSpec, FakeStore, spec};

use super::{distinct_keys, input_of, key_of, tried};

fn drv(store: &FakeStore, spec: &DrvSpec) -> Drv {
    read(store.dir(), &store.add(spec)).expect("readable drv")
}

fn sets(variants: &[&str]) -> Vec<String> {
    variants.iter().map(|&variant| variant.to_owned()).collect()
}

#[test]
fn a_root_is_looked_up_by_pname_in_the_sets_its_platform_and_cuda_call_for() {
    let store = FakeStore::new();
    let cudart = store.add(&spec("cuda_cudart-12.8"));
    let plain = drv(&store, &spec("hello-2.12").env("pname", "hello"));
    let cuda = drv(
        &store,
        &spec("llama-cpp-0.5.0")
            .env("pname", "llama-cpp")
            .input(&cudart, &["out"]),
    );
    let mut i686 = drv(&store, &spec("perl-IO-Tty").env("pname", "IO-Tty"));
    i686.system = "i686-linux".to_owned();
    let mut arm = plain.clone();
    arm.system = "aarch64-linux".to_owned();
    assert_eq!(
        key_of(&plain, "x86_64-linux"),
        Ok(("hello".to_owned(), sets(&[""])))
    );
    assert_eq!(
        key_of(&cuda, "x86_64-linux"),
        Ok(("llama-cpp".to_owned(), sets(&["", "pkgsCuda"])))
    );
    assert_eq!(
        key_of(&i686, "x86_64-linux"),
        Ok(("IO-Tty".to_owned(), sets(&["pkgsi686Linux"])))
    );
    assert_eq!(
        key_of(&arm, "x86_64-linux"),
        Err("it builds for aarch64-linux, and only x86_64-linux is looked up".to_owned())
    );
}

#[test]
fn fetches_and_glue_are_not_looked_up_and_keys_are_distinct() {
    let store = FakeStore::new();
    let fetch = drv(&store, &spec("src.tar.gz").fixed().env("pname", "src"));
    let local = drv(
        &store,
        &spec("node-24")
            .env("pname", "node")
            .env("allowSubstitutes", ""),
    );
    let glue = drv(&store, &spec("etc"));
    let hello = drv(&store, &spec("hello-2.12").env("pname", "hello"));
    let again = drv(
        &store,
        &spec("hello-2.12").env("pname", "hello").env("patches", "x"),
    );
    assert!(key_of(&fetch, "x86_64-linux").is_err());
    assert!(key_of(&glue, "x86_64-linux").is_err());
    assert!(key_of(&local, "x86_64-linux").is_err());
    assert_eq!(
        distinct_keys(&[&fetch, &glue, &hello, &again], "x86_64-linux"),
        [("hello".to_owned(), sets(&[""]))]
    );
}

#[tokio::test]
async fn a_root_is_an_input_of_each_name_that_takes_it_in_once_in_name_order() {
    let store = FakeStore::new();
    let glue = store.add(&spec("config.toml"));
    let files = store.add(&spec("home-manager-files").input(&glue, &["out"]));
    let unit = store.add(&spec("unit").input(&glue, &["out"]));
    let other_unit = store.add(&spec("unit").input(&glue, &["out"]).env("x", "1"));
    let top = store.add(
        &spec("system")
            .input(&files, &["out"])
            .input(&unit, &["out"])
            .input(&other_unit, &["out"]),
    );
    let graph = crate::walk::walk(
        store.dir(),
        &top,
        &crate::fixture_tests::FakeCache::default(),
        &|_| {},
    )
    .await;
    let found = input_of(&graph);
    assert_eq!(
        found.get(&glue).map(Vec::as_slice),
        Some(&["home-manager-files".to_owned(), "unit".to_owned()][..])
    );
    assert_eq!(found.get(&top), None);
}

#[test]
fn tried_spells_every_attribute_in_every_input_and_set() {
    let inputs = [
        Input {
            label: "nixpkgs".to_owned(),
            path: "/n".to_owned(),
        },
        Input {
            label: "unstable".to_owned(),
            path: "/u".to_owned(),
        },
    ];
    let lookup = Lookup {
        pname: "llama-cpp".to_owned(),
        attrs: vec![
            vec!["llama-cpp".to_owned()],
            vec!["python3Packages".to_owned(), "llama-cpp".to_owned()],
        ],
        variants: sets(&["", "pkgsCuda"]),
    };
    assert_eq!(
        tried(&inputs, &lookup),
        [
            "nixpkgs#llama-cpp",
            "nixpkgs#pkgsCuda.llama-cpp",
            "unstable#llama-cpp",
            "unstable#pkgsCuda.llama-cpp",
            "nixpkgs#python3Packages.llama-cpp",
            "nixpkgs#pkgsCuda.python3Packages.llama-cpp",
            "unstable#python3Packages.llama-cpp",
            "unstable#pkgsCuda.python3Packages.llama-cpp",
        ]
    );
}

const FAKE_NIXPKGS: &str = r#"
{ localSystem, config ? { }, overlays ? [ ] }:
let
  mk = name: env: meta: derivation ({ inherit name; builder = "/bin/sh"; system = localSystem; } // env) // { inherit meta; };
in
{
  hello = mk "hello-2.12" { } { };
  app = mk "app-1.0" { pname = "app"; version = "1.0"; } { hydraPlatforms = [ localSystem ]; };
  cacert = mk "nss-cacert-3.126" { pname = "nss-cacert"; version = "3.126"; } { hydraPlatforms = [ localSystem ]; };
}
"#;

async fn instantiate(dir: &std::path::Path, attr: &str) -> nix_derivation::StorePath {
    let expr = format!(
        "(import {} {{ localSystem = \"x86_64-linux\"; }}).{attr}.drvPath",
        dir.display()
    );
    let drv = crate::eval::nix(&["eval", "--impure", "--raw", "--expr", &expr])
        .await
        .expect("instantiates");
    nix_derivation::StorePath::from_absolute_path(drv.as_bytes()).expect("a drv path")
}

/// Real nix, a fake nixpkgs input, the real store and a `file://` cache:
/// the root is nixpkgs' own `app`, so its `hello` decides the category.
async fn classify_app(hello_served: bool) -> super::Reason {
    classified_app(hello_served).await.0
}

/// The same, with the progress the comparison reported.
async fn classified_app(hello_served: bool) -> (super::Reason, Vec<crate::Progress>) {
    classified("app", hello_served).await
}

/// The same for the root nixpkgs builds at `attr`.
async fn classified(attr: &str, hello_served: bool) -> (super::Reason, Vec<crate::Progress>) {
    let nixpkgs = tempfile::tempdir().expect("tempdir");
    std::fs::write(nixpkgs.path().join("default.nix"), FAKE_NIXPKGS).expect("write");
    std::fs::create_dir_all(nixpkgs.path().join("pkgs/top-level")).expect("mkdir");
    std::fs::write(
        nixpkgs.path().join("pkgs/top-level/all-packages.nix"),
        "{ }",
    )
    .expect("write");
    let root = instantiate(nixpkgs.path(), attr).await;
    let hello = instantiate(nixpkgs.path(), "hello").await;
    let store = std::path::Path::new("/nix/store");
    let cache = tempfile::tempdir().expect("tempdir");
    let hello_out = read(store, &hello)
        .expect("hello")
        .outputs
        .into_values()
        .next()
        .expect("an out");
    let served = if hello_served {
        vec![(
            &hello_out,
            crate::fixture_tests::narinfo(&hello_out, Some(crate::fixture_tests::SECRET)),
        )]
    } else {
        vec![]
    };
    let url = crate::fixture_tests::file_cache(cache.path(), &served);
    let substituters =
        crate::cache::Substituters::new(&[url], &[crate::fixture_tests::PUBLIC.to_owned()], true)
            .expect("substituters");
    let graph = crate::walk::walk(
        store,
        &root,
        &crate::fixture_tests::FakeCache::default(),
        &|_| {},
    )
    .await;
    let split = crate::roots::split(&graph);
    let labels =
        crate::modules::Labels::parse(&format!(r#"{{"path": "{}"}}"#, nixpkgs.path().display()))
            .expect("labels");
    let heard = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&heard);
    let options = crate::Options::new("self")
        .with_store_dir(store)
        .with_cache_dir(cache.path())
        .with_progress(move |progress| sink.lock().expect("lock").push(progress));
    let (reasons, warnings) = super::classify_roots(
        &options,
        &graph,
        &split,
        super::Inputs::Listed(&labels),
        &substituters,
    )
    .await;
    assert!(warnings.is_empty(), "{warnings:?}");
    let reason = reasons
        .get(&root.to_absolute_path())
        .cloned()
        .expect("the root is classified");
    let heard = heard.lock().expect("lock").clone();
    (reason, heard)
}

#[tokio::test]
async fn an_unchanged_root_whose_hello_is_missing_is_newer_than_the_cache() {
    assert_eq!(
        classify_app(false).await,
        super::Reason::NewerThanCache {
            against: "self#app".to_owned()
        }
    );
}

#[tokio::test]
async fn an_unchanged_root_whose_hello_is_cached_is_unknown() {
    assert!(matches!(
        classify_app(true).await,
        super::Reason::Unknown { because } if because.starts_with("self#app builds exactly this")
    ));
}

#[tokio::test]
async fn comparing_with_nixpkgs_says_how_many_roots_it_compares() {
    let (_, heard) = classified_app(false).await;
    assert_eq!(heard, [crate::Progress::Comparing { roots: 1 }]);
}

#[tokio::test]
async fn a_root_whose_attribute_differs_from_its_pname_is_compared_with_it() {
    let (reason, heard) = classified("cacert", false).await;
    assert_eq!(
        reason,
        super::Reason::NewerThanCache {
            against: "self#cacert".to_owned()
        }
    );
    assert_eq!(
        heard,
        [
            crate::Progress::Comparing { roots: 1 },
            crate::Progress::IndexingAttributes {
                input: "self".to_owned()
            }
        ]
    );
}
