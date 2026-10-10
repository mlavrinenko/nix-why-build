use super::{Candidate, Input, Lookup, attr_guesses, find, nix_string};

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|&part| part.to_owned()).collect()
}

#[test]
fn guesses_the_pname_its_camel_case_and_the_language_sets() {
    assert_eq!(
        attr_guesses("glibc-locales"),
        [
            path(&["glibc-locales"]),
            path(&["glibcLocales"]),
            path(&["python3Packages", "glibc-locales"]),
            path(&["perlPackages", "Glibclocales"]),
        ]
    );
    assert_eq!(
        attr_guesses("IO-Tty"),
        [
            path(&["IO-Tty"]),
            path(&["IOTty"]),
            path(&["python3Packages", "IO-Tty"]),
            path(&["perlPackages", "IOTty"]),
        ]
    );
    assert_eq!(
        attr_guesses("nh"),
        [
            path(&["nh"]),
            path(&["python3Packages", "nh"]),
            path(&["perlPackages", "Nh"])
        ]
    );
}

#[test]
fn nix_strings_escape_what_nix_would_interpolate() {
    assert_eq!(nix_string(r#"a"b\c${d}$e"#), r#""a\"b\\c\${d}$e""#);
}

#[test]
fn a_candidate_labels_itself_by_input_variant_and_attribute() {
    let plain = Candidate {
        input: "nixpkgs".to_owned(),
        attr: "hello".to_owned(),
        ..Candidate::default()
    };
    let cuda = Candidate {
        variant: "pkgsCuda".to_owned(),
        input: "unstable".to_owned(),
        attr: "llama-cpp".to_owned(),
        ..Candidate::default()
    };
    assert_eq!(plain.label(), "nixpkgs#hello");
    assert_eq!(cuda.label(), "unstable#pkgsCuda.llama-cpp");
}

const FAKE_NIXPKGS: &str = r#"
{ localSystem, config ? { }, overlays ? [ ] }:
let
  mk = name: attrs: derivation { inherit name; builder = "/bin/sh"; system = localSystem; } // attrs;
in
{
  hello = mk "hello-2.12" { };
  llama-cpp = mk "llama-cpp-0.5.0" { version = "0.5.0"; };
  corefonts = mk "corefonts-1" { version = "1"; meta = { unfree = true; hydraPlatforms = [ ]; }; };
  pkgsCuda.llama-cpp = mk "llama-cpp-0.5.0" { version = "0.5.0"; cudaSupport = true; };
  perlPackages.IOTty = mk "perl-IO-Tty-1.20" { version = "1.20"; meta.broken = true; };
  aliased = throw "aliased away";
}
"#;

/// The expression itself, run by the real `nix` against a fake nixpkgs.
async fn found() -> super::Found {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("default.nix"), FAKE_NIXPKGS).expect("write fake nixpkgs");
    let inputs = [Input {
        label: "fake".to_owned(),
        path: dir.path().display().to_string(),
    }];
    let lookups = [
        Lookup {
            attrs: attr_guesses("llama-cpp"),
            variants: vec![String::new(), "pkgsCuda".to_owned()],
        },
        Lookup {
            attrs: attr_guesses("corefonts"),
            variants: vec![String::new()],
        },
        Lookup {
            attrs: attr_guesses("IO-Tty"),
            variants: vec![String::new()],
        },
        Lookup {
            attrs: vec![path(&["aliased"]), path(&["absent"])],
            variants: vec![String::new(), "pkgsCuda".to_owned()],
        },
    ];
    find("x86_64-linux", &inputs, &lookups)
        .await
        .expect("nix evaluates the lookup")
}

#[tokio::test]
async fn finds_every_candidate_in_every_input() {
    let found = found().await;
    let labels: Vec<Vec<String>> = found
        .roots
        .iter()
        .map(|candidates| candidates.iter().map(Candidate::label).collect())
        .collect();
    assert_eq!(
        labels,
        [
            vec![
                "fake#llama-cpp".to_owned(),
                "fake#pkgsCuda.llama-cpp".to_owned()
            ],
            vec!["fake#corefonts".to_owned()],
            vec!["fake#perlPackages.IOTty".to_owned()],
            vec![],
        ]
    );
}

#[tokio::test]
async fn candidates_carry_meta_and_each_input_its_hello() {
    let found = found().await;
    let corefonts = found
        .roots
        .get(1)
        .and_then(|candidates| candidates.first())
        .expect("corefonts");
    assert!(corefonts.unfree && !corefonts.broken);
    assert_eq!(corefonts.hydra_platforms, Some(vec![]));
    assert_eq!(corefonts.version.as_deref(), Some("1"));
    assert!(
        corefonts.drv.ends_with("-corefonts-1.drv"),
        "{}",
        corefonts.drv
    );
    let tty = found
        .roots
        .get(2)
        .and_then(|candidates| candidates.first())
        .expect("IO-Tty");
    assert!(tty.broken && tty.hydra_platforms.is_none());
    let [hello] = found.hello.as_slice() else {
        panic!("one hello: {:?}", found.hello);
    };
    assert_eq!(hello.input, "fake");
    assert!(
        hello
            .drv
            .as_deref()
            .is_some_and(|drv| drv.ends_with("-hello-2.12.drv"))
    );
}

#[tokio::test]
async fn an_input_that_does_not_import_is_skipped() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("default.nix"), "throw \"not nixpkgs\"").expect("write");
    let inputs = [Input {
        label: "broken".to_owned(),
        path: dir.path().display().to_string(),
    }];
    let found = find(
        "x86_64-linux",
        &inputs,
        &[Lookup {
            attrs: attr_guesses("hello"),
            variants: vec![String::new()],
        }],
    )
    .await
    .expect("nix evaluates");
    assert!(found.hello.is_empty());
    assert_eq!(found.roots, [vec![]]);
}

#[tokio::test]
async fn a_nix_failure_is_one_line() {
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
