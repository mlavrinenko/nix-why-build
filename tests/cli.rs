use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn greets_provided_name() {
    Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("binary should build")
        .arg("Ferris")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello from Ferris!"));
}

#[test]
fn reports_version() {
    Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("binary should build")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
