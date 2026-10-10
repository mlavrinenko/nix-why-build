//! When the CLI colors the report.

use assert_cmd::Command;

#[path = "support/chain.rs"]
mod chain;
#[path = "support/fixture.rs"]
mod fixture;

use chain::Chain;
use fixture::PUBLIC;

const BOLD_APP: &str = "\x1b[1mapp-1.0\x1b[0m";

/// The CLI run over the chain with `extra` flags, `NO_COLOR` set or not;
/// its stdout is a pipe, as in a script.
fn stdout(extra: &[&str], no_color: bool) -> String {
    let chain = Chain::new();
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).expect("binary should build");
    cmd.arg("--store-dir")
        .arg(chain.store.dir())
        .arg(chain.top.to_absolute_path())
        .args(["--substituter", &chain.url(), "--trusted-key", PUBLIC])
        .args(extra)
        .env_remove("NO_COLOR");
    if no_color {
        cmd.env("NO_COLOR", "1");
    }
    let output = cmd.output().expect("runs");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("utf-8")
}

#[test]
fn a_pipe_gets_plain_text() {
    assert_eq!(
        stdout(&[], false),
        "app-1.0\n  why unknown: no nixpkgs flake input to compare against\n"
    );
}

#[test]
fn always_colors_even_into_a_pipe() {
    let colored = stdout(&["--color", "always"], false);
    assert!(colored.starts_with(BOLD_APP), "{colored:?}");
}

#[test]
fn an_explicit_always_beats_no_color() {
    let colored = stdout(&["--color", "always"], true);
    assert!(colored.starts_with(BOLD_APP), "{colored:?}");
}

#[test]
fn never_prints_plain_text() {
    assert_eq!(stdout(&["--color", "never"], false), stdout(&[], false));
}

#[test]
fn json_is_never_colored() {
    let json = stdout(&["--json", "--color", "always"], false);
    assert!(!json.contains('\x1b'), "{json:?}");
}

#[test]
fn an_unknown_choice_is_refused() {
    let chain = Chain::new();
    Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("binary should build")
        .arg(chain.top.to_absolute_path())
        .args(["--color", "sometimes"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "possible values: auto, always, never",
        ));
}
