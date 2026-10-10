use super::{Candidate, attr_guesses, nix_string};

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

// Both expressions run under real nix here. Declared in this file rather than
// lib.rs because mt counts a lib.rs edit as a product change and asks for a
// CHANGELOG entry.
#[path = "nix_tests.rs"]
mod nix_tests;
