use std::path::Path;

use assert_cmd::Command;
use nix_compat::store_path::StorePath;
use predicates::prelude::*;

#[path = "support/fixture.rs"]
mod fixture;

use fixture::{FakeStore, PUBLIC, SECRET, file_cache, narinfo, spec};

/// A store where `app` and its glue miss and `lib` is served:
/// `system` (glue) -> `app-1.0` -> `lib-1.0` (served).
struct World {
    store: FakeStore,
    cache: tempfile::TempDir,
    top: StorePath,
}

fn world() -> World {
    let store = FakeStore::new();
    let lib = store.add(&spec("lib-1.0").env("pname", "lib"));
    let app = store.add(
        &spec("app-1.0")
            .env("pname", "app")
            .env("version", "1.0")
            .input(&lib, &["out"]),
    );
    let top = store.add(&spec("system").input(&app, &["out"]));
    let cache = tempfile::tempdir().expect("tempdir");
    let lib_out = store.output(&lib, "out");
    file_cache(cache.path(), &[(&lib_out, narinfo(&lib_out, Some(SECRET)))]);
    World { store, cache, top }
}

fn cmd(world: &World) -> Command {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).expect("binary should build");
    cmd.arg("--store-dir").arg(world.store.dir());
    cmd
}

fn cache_url(world: &World) -> String {
    format!("file://{}", world.cache.path().display())
}

/// A `nix` on `PATH` that answers `config show --json` and `eval`, logging
/// its arguments to `args.log`.
fn fake_nix(dir: &Path, drv: &str, config: &str) -> String {
    let script = format!(
        "#!/bin/sh\necho \"$@\" >> {log}\ncase \"$1\" in\n  config) printf '%s' '{config}' ;;\n  eval) printf '%s' '{drv}' ;;\nesac\n",
        log = dir.join("args.log").display(),
    );
    install_nix(dir, &script)
}

/// Writes an executable `nix` script into `dir`; returns a `PATH` that finds it first.
fn install_nix(dir: &Path, script: &str) -> String {
    let nix = dir.join("nix");
    std::fs::write(&nix, script).expect("write fake nix");
    let mut perms = std::fs::metadata(&nix).expect("stat").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&nix, perms).expect("chmod");
    format!(
        "{}:{}",
        dir.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

#[test]
fn lists_each_uncached_package_and_hides_glue() {
    let world = world();
    cmd(&world)
        .arg(world.top.to_absolute_path())
        .args(["--substituter", &cache_url(&world), "--trusted-key", PUBLIC])
        .assert()
        .success()
        .stdout("app-1.0\n")
        .stderr("1 to build, plus 1 config derivation\n");
}

/// The value at a JSON pointer, `Null` when absent.
fn at(value: &serde_json::Value, pointer: &str) -> serde_json::Value {
    value.pointer(pointer).cloned().unwrap_or_default()
}

#[test]
fn prints_the_report_as_json() {
    let world = world();
    let output = cmd(&world)
        .arg(world.top.to_absolute_path())
        .args([
            "--json",
            "--substituter",
            &cache_url(&world),
            "--trusted-key",
            PUBLIC,
        ])
        .output()
        .expect("runs");
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(at(&report, "/toplevel"), world.top.to_absolute_path());
    assert_eq!(
        at(&report, "/substituters"),
        serde_json::json!([cache_url(&world)])
    );
    assert_eq!(at(&report, "/to_build/0/name"), "app-1.0");
    assert_eq!(at(&report, "/to_build/0/pname"), "app");
    assert_eq!(at(&report, "/to_build/0/version"), "1.0");
    assert_eq!(at(&report, "/to_build/1/name"), "system");
    assert_eq!(at(&report, "/to_build/1/pname"), serde_json::Value::Null);
    assert_eq!(at(&report, "/unknown"), serde_json::json!([]));
}

#[test]
fn evaluates_a_nixos_configuration_and_reads_nix_config() {
    let world = world();
    let bin = tempfile::tempdir().expect("tempdir");
    let config = format!(
        r#"{{"substituters":{{"value":["{}"]}},"trusted-public-keys":{{"value":["{PUBLIC}"]}},"require-sigs":{{"value":true}}}}"#,
        cache_url(&world)
    );
    let path = fake_nix(bin.path(), &world.top.to_absolute_path(), &config);
    cmd(&world)
        .env("PATH", path)
        .arg(".#nixosConfigurations.box")
        .assert()
        .success()
        .stdout("app-1.0\n");
    let log = std::fs::read_to_string(bin.path().join("args.log")).expect("log");
    assert!(log.contains("config show --json"), "{log}");
    assert!(
        log.contains("eval --raw .#nixosConfigurations.box.config.system.build.toplevel.drvPath"),
        "{log}"
    );
}

#[test]
fn a_failing_evaluation_exits_non_zero_with_nix_stderr() {
    let world = world();
    let bin = tempfile::tempdir().expect("tempdir");
    let path = install_nix(
        bin.path(),
        "#!/bin/sh\necho 'error: attribute missing' >&2\nexit 1\n",
    );
    cmd(&world)
        .env("PATH", path)
        .args([
            ".#nope",
            "--substituter",
            &cache_url(&world),
            "--trusted-key",
            PUBLIC,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: attribute missing"));
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

#[test]
fn a_substituter_flag_alone_still_takes_keys_from_nix() {
    let world = world();
    let bin = tempfile::tempdir().expect("tempdir");
    let config = format!(r#"{{"trusted-public-keys":{{"value":["{PUBLIC}"]}}}}"#);
    let path = fake_nix(bin.path(), "unused", &config);
    cmd(&world)
        .env("PATH", path)
        .arg(world.top.to_absolute_path())
        .args(["--substituter", &cache_url(&world)])
        .assert()
        .success()
        .stdout("app-1.0\n");
}

#[test]
fn flags_alone_still_require_signatures() {
    let store = FakeStore::new();
    let lib = store.add(&spec("lib-1.0").env("pname", "lib"));
    let top = store.add(&spec("system").input(&lib, &["out"]));
    let cache = tempfile::tempdir().expect("tempdir");
    let lib_out = store.output(&lib, "out");
    let url = file_cache(cache.path(), &[(&lib_out, narinfo(&lib_out, None))]);
    Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .expect("binary should build")
        .arg("--store-dir")
        .arg(store.dir())
        .arg(top.to_absolute_path())
        .args(["--substituter", &url, "--trusted-key", PUBLIC])
        .assert()
        .success()
        .stdout("lib-1.0\n");
}
