use std::path::Path;

use assert_cmd::Command;
use nix_derivation::StorePath;
use predicates::prelude::*;

use crate::fixture::{FakeStore, PUBLIC, SECRET, file_cache, narinfo, spec};

/// A store where `lib` is served and everything else misses:
/// `system` (glue) -> `system-path` -> `app-1.0` -> `lib-1.0` (served),
/// `system` -> `tool-2.0` -> `src.tar.gz` (a fetch), and
/// `system` -> `unit-x.service` (glue, a root that drags in no package).
struct World {
    store: FakeStore,
    cache: tempfile::TempDir,
    top: StorePath,
    app: StorePath,
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
    let src = store.add(
        &spec("src.tar.gz")
            .fixed()
            .env("url", "https://example.org/src.tar.gz"),
    );
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
    World {
        store,
        cache,
        top,
        app,
    }
}

/// What `world()` prints: the fetch root first, since it drags a package in.
const GROUPED: &str = "src.tar.gz (fetch)\n  why fetch: no cache has https://example.org/src.tar.gz\n  drags in tool-2.0\napp-1.0\n  why unknown: no nixpkgs flake input to compare against\n  wanted by environment.systemPackages\n";

fn cmd(world: &World) -> Command {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).expect("binary should build");
    cmd.arg("--store-dir").arg(world.store.dir());
    cmd
}

fn cache_url(world: &World) -> String {
    format!("file://{}", world.cache.path().display())
}

/// Shell line for a fake `nix` to drop the `--extra-experimental-features`
/// pair, so `$1` is the subcommand.
const DROP_FEATURES: &str = "[ \"$1\" = --extra-experimental-features ] && shift 2\n";

/// Shell lines opening a fake `nix`: log the first line of the call to `log`,
/// then drop the features pair.
fn nix_prologue(log: &Path) -> String {
    format!(
        "printf '%s\\n' \"$*\" | head -n 1 >> {}\n{DROP_FEATURES}",
        log.display()
    )
}

/// A `nix` on `PATH` that answers `config show --json` and `path-info`, logging
/// the first line of each call to `args.log`.
/// `eval` (the option index) prints `index`, `flake archive`
/// prints a flake whose source is `/nix/store/s-source`.
fn fake_nix(dir: &Path, drv: &str, config: &str, index: &str) -> String {
    let script = format!(
        "#!/bin/sh\n{prologue}case \"$1\" in\n  config) printf '%s' '{config}' ;;\n  flake) printf '%s' '{{\"path\":\"/nix/store/s-source\"}}' ;;\n  path-info) printf '%s\\n' '{drv}' ;;\n  eval) printf '%s' '{index}' ;;\nesac\n",
        prologue = nix_prologue(&dir.join("args.log")),
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
        serde_json::json!([{"option": "environment.systemPackages", "via": "system-path", "via_drv": at(&report, "/roots/1/requested_by/0/via_drv"), "item": "app-1.0", "item_drv": at(&report, "/roots/1/drv"), "files": []}])
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
    let index = format!(
        r#"{{"scopes":[{{"prefix":"","glue":[],"lists":{{"environment.systemPackages":[{{"file":"/nix/store/s-source/apps/app.nix","drvs":["{}"]}}]}},"attrs":{{}},"sources":{{}}}}]}}"#,
        world.app.to_absolute_path()
    );
    let path = fake_nix(bin.path(), &world.top.to_absolute_path(), &config, &index);
    cmd(&world)
        .env("PATH", path)
        .arg(".#nixosConfigurations.box")
        .assert()
        .success()
        .stdout(GROUPED.replace(
            "environment.systemPackages",
            "environment.systemPackages (apps/app.nix)",
        ))
        .stderr("2 roots, 2 packages to build, 4 more without a pname\n");
    let log = std::fs::read_to_string(bin.path().join("args.log")).expect("log");
    assert!(log.contains("config show --json"), "{log}");
    assert!(
        log.contains(
            "path-info --derivation .#nixosConfigurations.box.config.system.build.toplevel"
        ),
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
    let path = fake_nix(bin.path(), "unused", &config, "{}");
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
        .stdout("lib-1.0\n  why unknown: no nixpkgs flake input to compare against\n");
}

#[test]
fn a_plain_installable_skips_the_option_index_but_not_its_flake() {
    let world = world();
    let bin = tempfile::tempdir().expect("tempdir");
    let path = fake_nix(bin.path(), &world.top.to_absolute_path(), "{}", "{}");
    cmd(&world)
        .env("PATH", path)
        .args([
            ".#packages.x86_64-linux.default",
            "--substituter",
            &cache_url(&world),
            "--trusted-key",
            PUBLIC,
        ])
        .assert()
        .success()
        .stdout(GROUPED);
    let log = std::fs::read_to_string(bin.path().join("args.log")).expect("log");
    assert!(!log.contains("--apply"), "{log}");
    assert!(log.contains("flake archive --json --dry-run ."), "{log}");
}

#[test]
fn a_failing_option_index_is_a_warning_not_a_failure() {
    let world = world();
    let bin = tempfile::tempdir().expect("tempdir");
    let script = format!(
        "#!/bin/sh\n{DROP_FEATURES}case \"$*\" in\n  *--apply*) printf 'evaluating...\\nerror: infinite recursion encountered\\n  at x.nix\\n' >&2; exit 1 ;;\n  path-info*) printf '%s\\n' '{}' ;;\n  flake*) printf '%s' '{{\"path\":\"/s\"}}' ;;\nesac\n",
        world.top.to_absolute_path()
    );
    let path = install_nix(bin.path(), &script);
    cmd(&world)
        .env("PATH", path)
        .args([
            ".#nixosConfigurations.box",
            "--substituter",
            &cache_url(&world),
            "--trusted-key",
            PUBLIC,
        ])
        .assert()
        .success()
        .stdout(GROUPED)
        .stderr(predicate::str::starts_with(
            "warning: module files unknown: error: infinite recursion encountered\n",
        ));
}

#[test]
fn every_nix_call_enables_the_features_it_needs() {
    let world = world();
    let bin = tempfile::tempdir().expect("tempdir");
    let index = r#"{"scopes":[]}"#;
    let path = fake_nix(bin.path(), &world.top.to_absolute_path(), "{}", index);
    cmd(&world)
        .env("PATH", path)
        .args([
            ".#nixosConfigurations.box",
            "--substituter",
            &cache_url(&world),
            "--trusted-key",
            PUBLIC,
        ])
        .assert()
        .success();
    let log = std::fs::read_to_string(bin.path().join("args.log")).expect("log");
    assert!(log.lines().count() >= 3, "{log}");
    for call in log.lines() {
        assert!(
            call.starts_with("--extra-experimental-features nix-command flakes "),
            "{call}"
        );
    }
}

/// Runs `installable` against a fake nix that answers with `world`'s top
/// `.drv`; returns the calls it logged.
fn nix_calls(world: &World, installable: &str) -> String {
    let bin = tempfile::tempdir().expect("tempdir");
    let path = fake_nix(bin.path(), &world.top.to_absolute_path(), "{}", "{}");
    cmd(world)
        .env("PATH", path)
        .args([
            installable,
            "--substituter",
            &cache_url(world),
            "--trusted-key",
            PUBLIC,
        ])
        .assert()
        .success()
        .stdout(GROUPED);
    std::fs::read_to_string(bin.path().join("args.log")).unwrap_or_default()
}

#[test]
fn a_flake_installable_without_a_fragment_is_handed_to_nix_whole() {
    let log = nix_calls(&world(), ".");
    assert!(log.contains("path-info --derivation .\n"), "{log}");
    assert!(log.contains("flake archive --json --dry-run .\n"), "{log}");
}

#[test]
fn a_quoted_configuration_name_with_dots_still_resolves_to_its_toplevel() {
    let log = nix_calls(&world(), r#".#nixosConfigurations."a.b""#);
    assert!(
        log.contains(
            r#"path-info --derivation .#nixosConfigurations."a.b".config.system.build.toplevel"#
        ),
        "{log}"
    );
    assert!(
        log.contains(r#"eval --json .#nixosConfigurations."a.b" --apply"#),
        "{log}"
    );
}

#[test]
fn a_drv_path_runs_without_asking_nix_anything() {
    let world = world();
    let log = nix_calls(&world, &world.top.to_absolute_path());
    assert_eq!(log, "");
}

/// The binary as a plain `std` command, whose stdio the test can wire up.
fn plain_cmd(world: &World) -> std::process::Command {
    let mut cmd = std::process::Command::new(cmd(world).get_program());
    cmd.arg("--store-dir")
        .arg(world.store.dir())
        .arg(world.top.to_absolute_path())
        .args(["--substituter", &cache_url(world), "--trusted-key", PUBLIC]);
    cmd
}

#[test]
fn a_closed_stdout_ends_the_run_quietly() {
    let world = world();
    let mut child = plain_cmd(&world)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    drop(child.stdout.take());
    let output = child.wait_with_output().expect("wait");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Broken pipe"), "{stderr}");
    assert!(output.status.success(), "{:?} {stderr}", output.status);
}

#[test]
fn any_other_write_error_still_fails_loudly() {
    let world = world();
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full");
    let output = plain_cmd(&world).stdout(full).output().expect("run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("No space left on device"), "{stderr}");
}

#[test]
fn flake_inputs_that_cannot_be_listed_are_a_warning_and_the_reason() {
    let world = world();
    let bin = tempfile::tempdir().expect("tempdir");
    let script = format!(
        "#!/bin/sh\n{DROP_FEATURES}case \"$1\" in\n  path-info) printf '%s\\n' '{}' ;;\n  flake) echo 'error: boom' >&2; exit 1 ;;\nesac\n",
        world.top.to_absolute_path()
    );
    let path = install_nix(bin.path(), &script);
    cmd(&world)
        .env("PATH", path)
        .args([
            ".#packages.x86_64-linux.default",
            "--substituter",
            &cache_url(&world),
            "--trusted-key",
            PUBLIC,
        ])
        .assert()
        .success()
        .stdout(GROUPED.replace(
            "no nixpkgs flake input to compare against",
            "the flake inputs could not be listed: error: boom",
        ))
        .stderr(predicate::str::starts_with(
            "warning: flake inputs unknown: error: boom\n",
        ));
}

#[test]
fn an_unsupported_substituter_is_a_warning_and_the_run_goes_on() {
    let world = world();
    let args = [
        "--substituter",
        "s3://bucket",
        "--substituter",
        &cache_url(&world),
        "--trusted-key",
        PUBLIC,
    ];
    cmd(&world)
        .arg(world.top.to_absolute_path())
        .args(args)
        .assert()
        .success()
        .stdout(GROUPED)
        .stderr(
            "warning: s3://bucket: unsupported scheme, not asked\n2 roots, 2 packages to build, 4 more without a pname\n",
        );
    let output = cmd(&world)
        .arg(world.top.to_absolute_path())
        .args(args)
        .arg("--json")
        .output()
        .expect("runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(
        at(&report, "/substituters"),
        serde_json::json!([cache_url(&world)])
    );
    assert_eq!(
        at(&report, "/warnings"),
        serde_json::json!(["s3://bucket: unsupported scheme, not asked"])
    );
    assert_eq!(at(&report, "/unknown"), serde_json::json!([]));
}
