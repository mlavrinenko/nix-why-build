use std::path::Path;

use assert_cmd::Command;
use nix_compat::store_path::StorePath;
use predicates::prelude::*;

#[path = "support/fixture.rs"]
mod fixture;

use fixture::{FakeStore, PUBLIC, SECRET, file_cache, narinfo, spec};

/// A store where `lib` is served and everything else misses:
/// `system` (glue) -> `system-path` -> `app-1.0` -> `lib-1.0` (served),
/// `system` -> `tool-2.0` -> `src.tar.gz` (a fetch), and
/// `system` -> `unit-x.service` (glue, a root that drags in no package).
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
    let src = store.add(&spec("src.tar.gz").fixed());
    let tool = store.add(&spec("tool-2.0").env("pname", "tool").input(&src, &["out"]));
    let unit = store.add(&spec("unit-x.service"));
    let path = store.add(&spec("system-path").input(&app, &["out"]));
    let top = store.add(
        &spec("system")
            .input(&path, &["out"])
            .input(&tool, &["out"])
            .input(&unit, &["out"]),
    );
    let cache = tempfile::tempdir().expect("tempdir");
    let lib_out = store.output(&lib, "out");
    file_cache(cache.path(), &[(&lib_out, narinfo(&lib_out, Some(SECRET)))]);
    World { store, cache, top }
}

/// What `world()` prints: the fetch root first, since it drags a package in.
const GROUPED: &str =
    "src.tar.gz (fetch)\n  drags in tool-2.0\napp-1.0\n  wanted by environment.systemPackages\n";

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
fn groups_packages_under_their_roots_and_hides_glue() {
    let world = world();
    cmd(&world)
        .arg(world.top.to_absolute_path())
        .args(["--substituter", &cache_url(&world), "--trusted-key", PUBLIC])
        .assert()
        .success()
        .stdout(GROUPED)
        .stderr("2 roots, 2 packages to build, 4 more without a pname\n");
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
    assert_eq!(at(&report, "/to_build/1/name"), "src.tar.gz");
    assert_eq!(at(&report, "/to_build/1/pname"), serde_json::Value::Null);
    assert_eq!(at(&report, "/roots/0/name"), "src.tar.gz");
    assert_eq!(at(&report, "/roots/0/fixed_output"), true);
    assert_eq!(
        at(&report, "/roots/0/cascades"),
        serde_json::json!([
            {"name": "tool-2.0", "pname": "tool", "drv": at(&report, "/roots/0/cascades/0/drv"), "chain": ["tool-2.0", "src.tar.gz"]},
            {"name": "system", "pname": null, "drv": at(&report, "/roots/0/cascades/1/drv"), "chain": ["system", "tool-2.0", "src.tar.gz"]},
        ])
    );
    assert_eq!(at(&report, "/roots/1/name"), "app-1.0");
    assert_eq!(
        at(&report, "/roots/1/requested_by"),
        serde_json::json!([{"option": "environment.systemPackages", "via": "system-path", "item": "app-1.0", "item_drv": at(&report, "/roots/1/drv")}])
    );
    assert_eq!(at(&report, "/roots/2/name"), "unit-x.service");
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
        .stdout(GROUPED);
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
        .stdout(GROUPED);
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
