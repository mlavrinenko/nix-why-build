//! What `explain` reports while it works, and what the CLI draws of it.

use std::sync::{Arc, Mutex};

use assert_cmd::Command;
use nix_why_build::{Options, Progress};

use crate::chain::Chain;
use crate::fixture::PUBLIC;

#[tokio::test]
async fn explain_reports_each_phase_to_the_callback_and_skips_nixpkgs_without_a_flake() {
    let chain = Chain::new();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let options = Options::new(&chain.top.to_absolute_path())
        .with_substituters(vec![chain.url()])
        .with_trusted_keys(vec![PUBLIC.to_owned()])
        .with_store_dir(chain.store.dir())
        .with_progress(move |progress| sink.lock().expect("lock").push(progress));
    let report = nix_why_build::explain(&options).await.expect("explained");
    assert_eq!(report.roots.len(), 1);
    let walking = |level, derivations| Progress::Walking {
        level,
        derivations,
        caches: 1,
    };
    assert_eq!(
        *seen.lock().expect("lock"),
        vec![
            Progress::Evaluating {
                installable: chain.top.to_absolute_path()
            },
            walking(1, 1),
            walking(2, 2),
            walking(3, 3),
        ]
    );
}

#[tokio::test]
async fn explain_without_a_callback_still_works() {
    let chain = Chain::new();
    let options = Options::new(&chain.top.to_absolute_path())
        .with_substituters(vec![chain.url()])
        .with_trusted_keys(vec![PUBLIC.to_owned()])
        .with_store_dir(chain.store.dir());
    let report = nix_why_build::explain(&options).await.expect("explained");
    assert_eq!(report.roots.len(), 1);
}

#[test]
fn a_piped_stderr_carries_no_progress_line() {
    let chain = Chain::new();
    let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("binary should build")
        .arg("--store-dir")
        .arg(chain.store.dir())
        .arg(chain.top.to_absolute_path())
        .args(["--substituter", &chain.url(), "--trusted-key", PUBLIC])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf-8");
    assert!(!stderr.contains('\r'), "{stderr:?}");
    assert!(!stderr.contains('\x1b'), "{stderr:?}");
    assert!(!stderr.contains("evaluating"), "{stderr:?}");
}
