use crate::counterparts::{Input, Lookup};
use crate::drv::{Drv, read};
use crate::fixture_tests::{DrvSpec, FakeStore, spec};

use super::{distinct_keys, key_of, tried};

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
    let glue = drv(&store, &spec("etc"));
    let hello = drv(&store, &spec("hello-2.12").env("pname", "hello"));
    let again = drv(
        &store,
        &spec("hello-2.12").env("pname", "hello").env("patches", "x"),
    );
    assert!(key_of(&fetch, "x86_64-linux").is_err());
    assert!(key_of(&glue, "x86_64-linux").is_err());
    assert_eq!(
        distinct_keys(&[&fetch, &glue, &hello, &again], "x86_64-linux"),
        [("hello".to_owned(), sets(&[""]))]
    );
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
}
"#;

async fn instantiate(dir: &std::path::Path, attr: &str) -> nix_compat::store_path::StorePath {
    let expr = format!(
        "(import {} {{ localSystem = \"x86_64-linux\"; }}).{attr}.drvPath",
        dir.display()
    );
    let drv = crate::eval::nix(&["eval", "--impure", "--raw", "--expr", &expr])
        .await
        .expect("instantiates");
    nix_compat::store_path::StorePath::from_absolute_path(drv.as_bytes()).expect("a drv path")
}

/// Real nix, a fake nixpkgs input, the real store and a `file://` cache:
/// the root is nixpkgs' own `app`, so its `hello` decides the category.
async fn classify_app(hello_served: bool) -> super::Reason {
    let nixpkgs = tempfile::tempdir().expect("tempdir");
    std::fs::write(nixpkgs.path().join("default.nix"), FAKE_NIXPKGS).expect("write");
    std::fs::create_dir_all(nixpkgs.path().join("pkgs/top-level")).expect("mkdir");
    std::fs::write(
        nixpkgs.path().join("pkgs/top-level/all-packages.nix"),
        "{ }",
    )
    .expect("write");
    let app = instantiate(nixpkgs.path(), "app").await;
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
    let graph = crate::walk::walk(store, &app, &crate::fixture_tests::FakeCache::default()).await;
    let split = crate::roots::split(&graph);
    let labels =
        crate::modules::Labels::parse(&format!(r#"{{"path": "{}"}}"#, nixpkgs.path().display()))
            .expect("labels");
    let (reasons, warnings) =
        super::classify_roots(store, &graph, &split, Some(&labels), &substituters).await;
    assert!(warnings.is_empty(), "{warnings:?}");
    reasons
        .get(&app.to_absolute_path())
        .cloned()
        .expect("the root is classified")
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
