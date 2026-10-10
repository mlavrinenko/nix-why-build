//! The Nix expressions, run by the real `nix` against fixtures.
//!
//! `src/counterparts.nix` and `src/index.nix` are product code that
//! `include_str!` embeds, and every other test fakes `nix` with a script, so
//! a typo in either would pass. These call the real entry points
//! (`counterparts::find`, `modules::load`) on `tests/fixtures/`: a fake nixpkgs
//! and a fake configuration written in Nix from `builtins` alone, so nothing
//! touches the network. The tests sit in `nix_counterparts_tests.rs` and
//! `nix_index_tests.rs`.
//!
//! Reading a `.drvPath` writes the `.drv` into the store, so the tests run
//! against a scratch store: nextest's setup script
//! (`tests/support/nix-scratch-store.sh`, wired in `.config/nextest.toml`)
//! exports `NIX_REMOTE=local?root=target/nix-scratch` to them alone. Not
//! `nix eval --read-only`: that is a flag, and the entry points fix their own
//! arguments, so passing it would take a hook in product code, while an
//! environment variable reaches them unchanged. It also leaves the
//! instantiation the product relies on to run.

#[path = "nix_index_tests.rs"]
mod index;
#[path = "nix_counterparts_tests.rs"]
mod lookups;

/// Fails a test that would write to the real store.
fn scratch_store() {
    let store = std::env::var("NIX_REMOTE").unwrap_or_default();
    assert!(
        store.contains("nix-scratch"),
        "run these with `just test`: nextest's setup script exports the scratch store (NIX_REMOTE={store:?})"
    );
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|&item| item.to_owned()).collect()
}
