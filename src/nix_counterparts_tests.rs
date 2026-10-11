use std::sync::{Arc, Mutex};

use crate::counterparts::{Candidate, Found, Input, Lookup, call, find};
use crate::modules::Labels;
use crate::renamed::find_renamed;
use crate::{Options, Progress};

use super::{fixture, scratch_store, strings};

fn input(label: &str, dir: &str) -> Input {
    Input {
        label: label.to_owned(),
        path: fixture(dir),
    }
}

fn lookup(pname: &str, variants: &[&str]) -> Lookup {
    Lookup::new(pname, strings(variants))
}

async fn find_in(system: &str, inputs: &[Input], lookups: &[Lookup]) -> Found {
    scratch_store();
    find(system, inputs, lookups)
        .await
        .expect("nix evaluates the lookups")
}

/// The candidates of each lookup in the fixture nixpkgs, labelled `nixpkgs`.
async fn nixpkgs(lookups: &[Lookup]) -> Vec<Vec<Candidate>> {
    let inputs = [input("nixpkgs", "nixpkgs")];
    find_in("x86_64-linux", &inputs, lookups).await.roots
}

fn labels(candidates: &[Candidate]) -> Vec<String> {
    candidates.iter().map(Candidate::label).collect()
}

/// `candidate` without its `.drv`, after checking the `.drv` is `name`'s.
fn without_drv(candidate: &Candidate, name: &str) -> Candidate {
    let suffix = format!("-{name}.drv");
    assert!(
        candidate.drv.starts_with("/nix/store/") && candidate.drv.ends_with(&suffix),
        "{}",
        candidate.drv
    );
    Candidate {
        drv: String::new(),
        ..candidate.clone()
    }
}

#[tokio::test]
async fn a_pname_is_found_under_each_of_its_attribute_guesses() {
    let cases = [
        ("llama-cpp", strings(&["nixpkgs#llama-cpp"])),
        ("glibc-locales", strings(&["nixpkgs#glibcLocales"])),
        ("requests", strings(&["nixpkgs#python3Packages.requests"])),
        (
            "flask-cors",
            strings(&["nixpkgs#python3Packages.flask-cors"]),
        ),
        ("IO-Tty", strings(&["nixpkgs#perlPackages.IOTty"])),
        ("no-such-package", vec![]),
    ];
    let lookups: Vec<Lookup> = cases
        .iter()
        .map(|(pname, _)| lookup(pname, &[""]))
        .collect();
    let found: Vec<Vec<String>> = nixpkgs(&lookups)
        .await
        .iter()
        .map(|candidates| labels(candidates))
        .collect();
    let expected: Vec<Vec<String>> = cases.into_iter().map(|(_, labels)| labels).collect();
    assert_eq!(found, expected);
}

#[tokio::test]
async fn a_variant_looks_in_the_package_set_it_names() {
    let roots = nixpkgs(&[
        lookup("llama-cpp", &["", "pkgsCuda", "pkgsMusl"]),
        lookup("eglexternalplatform", &["pkgsi686Linux"]),
        lookup("eglexternalplatform", &[""]),
    ])
    .await;
    let [llama, egl, plain_egl] = roots.as_slice() else {
        panic!("one answer per lookup: {roots:?}");
    };
    assert_eq!(
        labels(llama),
        ["nixpkgs#llama-cpp", "nixpkgs#pkgsCuda.llama-cpp"],
        "a variant nixpkgs lacks is skipped"
    );
    assert_eq!(labels(egl), ["nixpkgs#pkgsi686Linux.eglexternalplatform"]);
    assert_eq!(plain_egl, &Vec::<Candidate>::new());
    let [plain, cuda] = llama.as_slice() else {
        panic!("two llama-cpp: {llama:?}");
    };
    assert_eq!(
        (plain.variant.as_str(), cuda.variant.as_str()),
        ("", "pkgsCuda")
    );
    assert_eq!(cuda.attr, "llama-cpp");
    assert_ne!(plain.drv, cuda.drv);
}

#[tokio::test]
async fn a_candidate_carries_its_version_and_what_meta_says() {
    let roots = nixpkgs(&[
        lookup("llama-cpp", &[""]),
        lookup("corefonts", &[""]),
        lookup("IO-Tty", &[""]),
        lookup("ffmpeg", &[""]),
        lookup("fragile", &[""]),
    ])
    .await;
    let firsts: Vec<&Candidate> = roots.iter().filter_map(|found| found.first()).collect();
    let [llama, corefonts, tty, ffmpeg, fragile] = firsts.as_slice() else {
        panic!("one candidate each: {roots:?}");
    };
    let expect = |attr: &str, version: Option<&str>| Candidate {
        input: "nixpkgs".to_owned(),
        attr: attr.to_owned(),
        version: version.map(str::to_owned),
        ..Candidate::default()
    };
    assert_eq!(
        without_drv(llama, "llama-cpp-0.5.0"),
        expect("llama-cpp", Some("0.5.0"))
    );
    assert_eq!(
        without_drv(corefonts, "corefonts-1"),
        Candidate {
            unfree: true,
            hydra_platforms: Some(vec![]),
            ..expect("corefonts", Some("1"))
        }
    );
    assert_eq!(
        without_drv(tty, "IO-Tty-1.20"),
        Candidate {
            broken: true,
            ..expect("perlPackages.IOTty", Some("1.20"))
        }
    );
    assert_eq!(
        without_drv(ffmpeg, "ffmpeg-7.1"),
        Candidate {
            hydra_platforms: Some(strings(&["x86_64-linux", "aarch64-linux"])),
            ..expect("ffmpeg", Some("7.1"))
        },
        "hydraPlatforms keeps its order"
    );
    assert_eq!(
        without_drv(fragile, "fragile-1"),
        expect("fragile", None),
        "a field that does not evaluate falls back"
    );
}

#[tokio::test]
async fn what_is_no_derivation_is_no_candidate() {
    let roots = nixpkgs(&[
        lookup("aliased", &[""]),
        lookup("notADerivation", &[""]),
        lookup("undrvable", &[""]),
    ])
    .await;
    assert_eq!(roots, [vec![], vec![], vec![]]);
}

#[tokio::test]
async fn nixpkgs_is_imported_for_the_system_asked() {
    let inputs = [input("nixpkgs", "nixpkgs")];
    let found = find_in("aarch64-linux", &inputs, &[lookup("system-probe", &[""])]).await;
    let version = found
        .roots
        .first()
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.version.as_deref());
    assert_eq!(version, Some("aarch64-linux"));
}

#[tokio::test]
async fn every_input_that_imports_answers_and_one_that_throws_is_skipped() {
    let inputs = [
        input("nixpkgs", "nixpkgs"),
        input("broken", "nixpkgs-throws"),
        input("unstable", "nixpkgs-unstable"),
    ];
    let found = find_in("x86_64-linux", &inputs, &[lookup("llama-cpp", &[""])]).await;
    let [roots] = found.roots.as_slice() else {
        panic!("one lookup: {found:?}");
    };
    let versions: Vec<(String, Option<&str>)> = roots
        .iter()
        .map(|candidate| (candidate.label(), candidate.version.as_deref()))
        .collect();
    assert_eq!(
        versions,
        [
            ("nixpkgs#llama-cpp".to_owned(), Some("0.5.0")),
            ("unstable#llama-cpp".to_owned(), Some("0.6.0")),
        ]
    );
    let hellos: Vec<(&str, bool)> = found
        .hello
        .iter()
        .map(|hello| {
            let suffix = if hello.input == "nixpkgs" {
                "-hello-2.12.drv"
            } else {
                "-hello-2.13.drv"
            };
            let drv = hello.drv.as_deref().unwrap_or_default();
            (hello.input.as_str(), drv.ends_with(suffix))
        })
        .collect();
    assert_eq!(hellos, [("nixpkgs", true), ("unstable", true)]);
}

#[tokio::test]
async fn the_inputs_labels_accepts_are_the_ones_the_lookup_imports() {
    let archive = format!(
        r#"{{"path": "/nix/store/none-source", "inputs": {{
            "nixpkgs": {{"path": "{}"}},
            "unstable": {{"path": "{}"}}
        }}}}"#,
        fixture("nixpkgs"),
        fixture("nixpkgs-unstable"),
    );
    let accepted = Labels::parse(&archive)
        .expect("an archive")
        .nixpkgs_inputs();
    assert_eq!(accepted, [input("nixpkgs", "nixpkgs")]);
    let found = find_in("x86_64-linux", &accepted, &[]).await;
    let hellos: Vec<&str> = found
        .hello
        .iter()
        .map(|hello| hello.input.as_str())
        .collect();
    assert_eq!(hellos, ["nixpkgs"]);
}

#[tokio::test]
async fn a_nix_failure_is_one_line() {
    scratch_store();
    let err = find(
        "x86_64-linux",
        &[Input {
            label: "x".to_owned(),
            path: "/does/not/exist".to_owned(),
        }],
        &[],
    )
    .await
    .expect_err("a missing path does not import");
    assert!(err.contains("error"), "{err}");
    assert!(!err.contains('\n'), "{err}");
}

#[tokio::test]
async fn five_thousand_lookups_evaluate_like_five() {
    let both = ["", "pkgsCuda"];
    let mut lookups = vec![lookup("llama-cpp", &both)];
    lookups.extend((1..4_999).map(|at| lookup(&format!("absent-package-{at}"), &both)));
    lookups.push(lookup("flask-cors", &both));
    let inputs = [input("nixpkgs", "nixpkgs")];
    let query = call("x86_64-linux", &inputs, &lookups).expect("serializes");
    assert!(
        query.stdin.len() > 128 * 1024,
        "{} bytes: more than one argument can hold",
        query.stdin.len()
    );
    let roots = find_in("x86_64-linux", &inputs, &lookups).await.roots;
    assert_eq!(roots.len(), 5_000);
    let labelled: Vec<Vec<String>> = roots.iter().map(|found| labels(found)).collect();
    assert_eq!(
        labelled.first(),
        Some(&strings(&[
            "nixpkgs#llama-cpp",
            "nixpkgs#pkgsCuda.llama-cpp"
        ]))
    );
    assert_eq!(
        labelled.last(),
        Some(&strings(&["nixpkgs#python3Packages.flask-cors"]))
    );
    assert!(labelled.iter().skip(1).take(4_998).all(Vec::is_empty));
}

/// Looks `lookups` up in `inputs`, then the misses again under the index
/// cached in `cache`: each lookup's labels, the warnings and the progress
/// heard.
async fn renamed_in(
    inputs: &[Input],
    lookups: &[Lookup],
    cache: &std::path::Path,
) -> (Vec<Vec<String>>, Vec<String>, Vec<Progress>) {
    let mut found = find_in("x86_64-linux", inputs, lookups).await;
    let heard = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&heard);
    let options = Options::new("unused")
        .with_cache_dir(cache)
        .with_progress(move |progress| sink.lock().expect("unpoisoned").push(progress));
    let warnings = find_renamed("x86_64-linux", inputs, lookups, &mut found, &options).await;
    let labelled = found.roots.iter().map(|found| labels(found)).collect();
    let heard = heard.lock().expect("unpoisoned").clone();
    (labelled, warnings, heard)
}

#[tokio::test]
async fn a_pname_no_guess_finds_is_found_under_the_attribute_that_carries_it() {
    let cache = tempfile::tempdir().expect("tempdir");
    let inputs = [input("nixpkgs", "nixpkgs")];
    let lookups = [
        lookup("nss-cacert", &[""]),
        lookup("no-such-package", &[""]),
    ];
    let (labelled, warnings, heard) = renamed_in(&inputs, &lookups, cache.path()).await;
    assert_eq!(labelled, [strings(&["nixpkgs#cacert"]), vec![]]);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(
        heard,
        [Progress::IndexingAttributes {
            input: "nixpkgs".to_owned()
        }]
    );
    let index = std::fs::read_to_string(cache.path().join("attrs/nixpkgs-x86_64-linux.json"))
        .expect("the index is cached");
    assert!(index.contains(r#""nss-cacert":["cacert"]"#), "{index}");
}

#[tokio::test]
async fn a_cached_index_is_read_back_instead_of_built() {
    let cache = tempfile::tempdir().expect("tempdir");
    let attrs = cache.path().join("attrs");
    std::fs::create_dir_all(&attrs).expect("mkdir");
    std::fs::write(
        attrs.join("nixpkgs-x86_64-linux.json"),
        r#"{"renamed-hello":["hello"]}"#,
    )
    .expect("seed the cache");
    let inputs = [input("nixpkgs", "nixpkgs")];
    let lookups = [lookup("renamed-hello", &[""])];
    let (labelled, _, heard) = renamed_in(&inputs, &lookups, cache.path()).await;
    assert_eq!(labelled, [strings(&["nixpkgs#hello"])]);
    assert_eq!(heard, []);
}

#[tokio::test]
async fn a_lookup_whose_guess_hits_builds_no_index() {
    let cache = tempfile::tempdir().expect("tempdir");
    let inputs = [input("nixpkgs", "nixpkgs")];
    let (labelled, _, heard) = renamed_in(&inputs, &[lookup("hello", &[""])], cache.path()).await;
    assert_eq!(labelled, [strings(&["nixpkgs#hello"])]);
    assert_eq!(heard, []);
    assert!(!cache.path().join("attrs").exists());
}

#[tokio::test]
async fn an_index_that_fails_to_build_is_a_warning() {
    scratch_store();
    let cache = tempfile::tempdir().expect("tempdir");
    let inputs = [Input {
        label: "x".to_owned(),
        path: "/does/not/exist".to_owned(),
    }];
    let mut found = Found {
        hello: vec![],
        roots: vec![vec![]],
    };
    let options = Options::new("unused").with_cache_dir(cache.path());
    let lookups = [lookup("nss-cacert", &[""])];
    let warnings = find_renamed("x86_64-linux", &inputs, &lookups, &mut found, &options).await;
    let [warning] = warnings.as_slice() else {
        panic!("one warning: {warnings:?}");
    };
    assert!(
        warning.starts_with("x's attributes unindexed: "),
        "{warning}"
    );
    assert!(!warning.contains('\n'), "{warning}");
    assert_eq!(found.roots, [Vec::<Candidate>::new()]);
}
