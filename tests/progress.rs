//! What `explain` reports while it works, and what the CLI draws of it.

use std::sync::{Arc, Mutex};

use assert_cmd::Command;
use nix_derivation::StorePath;
use nix_why_build::{Options, Progress};

#[path = "support/fixture.rs"]
mod fixture;

use fixture::{FakeStore, PUBLIC, SECRET, file_cache, narinfo, spec};

/// `top` -> `app-1.0` (missed, so a root) -> `lib-1.0` (served).
struct Chain {
    store: FakeStore,
    cache: tempfile::TempDir,
    top: StorePath,
}

impl Chain {
    fn new() -> Self {
        let store = FakeStore::new();
        let lib = store.add(&spec("lib-1.0").env("pname", "lib"));
        let app = store.add(&spec("app-1.0").env("pname", "app").input(&lib, &["out"]));
        let top = store.add(&spec("system").input(&app, &["out"]));
        let cache = tempfile::tempdir().expect("tempdir");
        let lib_out = store.output(&lib, "out");
        file_cache(cache.path(), &[(&lib_out, narinfo(&lib_out, Some(SECRET)))]);
        Self { store, cache, top }
    }

    fn url(&self) -> String {
        format!("file://{}", self.cache.path().display())
    }
}

#[tokio::test]
async fn explain_reports_each_phase_to_the_callback() {
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
            Progress::Comparing { roots: 1 },
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
