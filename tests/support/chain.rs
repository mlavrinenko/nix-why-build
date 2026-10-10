//! A three-derivation world for tests that need one root and one served input.
#![allow(dead_code)]

use nix_derivation::StorePath;

use super::fixture::{FakeStore, SECRET, file_cache, narinfo, spec};

/// `top` -> `app-1.0` (missed, so a root) -> `lib-1.0` (served).
pub struct Chain {
    pub store: FakeStore,
    pub top: StorePath,
    cache: tempfile::TempDir,
}

impl Chain {
    pub fn new() -> Self {
        let store = FakeStore::new();
        let lib = store.add(&spec("lib-1.0").env("pname", "lib"));
        let app = store.add(&spec("app-1.0").env("pname", "app").input(&lib, &["out"]));
        let top = store.add(&spec("system").input(&app, &["out"]));
        let cache = tempfile::tempdir().expect("tempdir");
        let lib_out = store.output(&lib, "out");
        file_cache(cache.path(), &[(&lib_out, narinfo(&lib_out, Some(SECRET)))]);
        Self { store, top, cache }
    }

    /// The cache that serves `lib-1.0`.
    pub fn url(&self) -> String {
        format!("file://{}", self.cache.path().display())
    }
}
