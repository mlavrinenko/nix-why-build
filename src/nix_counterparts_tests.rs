use crate::counterparts::{Candidate, Found, Input, Lookup, attr_guesses, find};
use crate::modules::Labels;

use super::{fixture, scratch_store, strings};

fn input(label: &str, dir: &str) -> Input {
    Input {
        label: label.to_owned(),
        path: fixture(dir),
    }
}

fn lookup(pname: &str, variants: &[&str]) -> Lookup {
    Lookup {
        attrs: attr_guesses(pname),
        variants: strings(variants),
    }
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
