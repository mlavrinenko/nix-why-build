use super::{Candidate, Input, Lookup, attr_guesses, call, nix_string};

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

/// `count` lookups, each of a distinct package, in two package sets.
fn lookups(count: usize) -> Vec<Lookup> {
    (0..count)
        .map(|at| Lookup {
            attrs: attr_guesses(&format!("package-number-{at}")),
            variants: vec![String::new(), "pkgsCuda".to_owned()],
        })
        .collect()
}

#[test]
fn the_arguments_stay_small_however_many_roots_are_looked_up() {
    let inputs = [Input {
        label: "nixpkgs".to_owned(),
        path: "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-source".to_owned(),
    }];
    let size = |count: usize| {
        let call = call("x86_64-linux", &inputs, &lookups(count)).expect("serializes");
        (call.args.iter().map(String::len).sum::<usize>(), call.stdin)
    };
    let (few, _) = size(5);
    let (many, stdin) = size(5_000);
    assert_eq!(many, few, "the arguments do not grow with the lookups");
    assert!(many < 16 * 1024, "{many} bytes of arguments");
    assert!(
        stdin.len() > 128 * 1024 && stdin.contains("package-number-4999"),
        "the query goes in on stdin, {} bytes of it, past what one argument can hold",
        stdin.len()
    );
}

// Both expressions run under real nix here. Declared in this file rather than
// lib.rs because mt counts a lib.rs edit as a product change and asks for a
// CHANGELOG entry.
#[path = "nix_tests.rs"]
mod nix_tests;
