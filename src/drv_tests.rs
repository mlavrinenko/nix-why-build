use std::collections::{BTreeMap, BTreeSet};

use crate::fixture_tests::{FakeStore, absent_drv, spec};

use super::{DrvError, read};

#[test]
fn reads_name_outputs_and_wanted_inputs() {
    let store = FakeStore::new();
    let lib = store.add(&spec("libfoo-1.0").outputs(&["out", "dev"]));
    let app = store.add(
        &spec("app-2.0")
            .outputs(&["out", "man"])
            .input(&lib, &["dev"]),
    );
    let drv = read(store.dir(), &app).expect("readable drv");
    assert_eq!(drv.path, app);
    assert_eq!(drv.name, "app-2.0");
    assert_eq!(
        drv.outputs.keys().map(String::as_str).collect::<Vec<_>>(),
        ["man", "out"]
    );
    assert_eq!(drv.outputs.get("man"), Some(&store.output(&app, "man")));
    assert_eq!(
        drv.inputs,
        BTreeMap::from([(lib, BTreeSet::from(["dev".to_owned()]))])
    );
    assert!(!drv.fixed_output);
    assert_eq!(drv.system, "x86_64-linux");
}

#[test]
fn takes_pname_and_version_from_env() {
    let store = FakeStore::new();
    let path = store.add(
        &spec("hello-2.12")
            .env("pname", "hello")
            .env("version", "2.12"),
    );
    let drv = read(store.dir(), &path).expect("readable drv");
    assert_eq!(drv.pname.as_deref(), Some("hello"));
    assert_eq!(drv.version.as_deref(), Some("2.12"));
}

#[test]
fn takes_pname_and_version_from_structured_attrs() {
    let store = FakeStore::new();
    let json = r#"{"pname":"llama-cpp","version":"0.5.0","outputs":{"out":"x"}}"#;
    let path = store.add(&spec("llama-cpp-0.5.0").env("__json", json));
    let drv = read(store.dir(), &path).expect("readable drv");
    assert_eq!(drv.pname.as_deref(), Some("llama-cpp"));
    assert_eq!(drv.version.as_deref(), Some("0.5.0"));
}

#[test]
fn glue_has_no_pname() {
    let store = FakeStore::new();
    let path = store.add(&spec("unit-sshd.service"));
    let drv = read(store.dir(), &path).expect("readable drv");
    assert_eq!(drv.pname, None);
    assert_eq!(drv.version, None);
}

#[test]
fn flags_a_fixed_output_derivation() {
    let store = FakeStore::new();
    let path = store.add(&spec("coreutils-9.11.tar.xz").fixed());
    assert!(read(store.dir(), &path).expect("readable drv").fixed_output);
}

#[test]
fn a_missing_file_is_a_read_error_naming_the_path() {
    let store = FakeStore::new();
    let err = read(store.dir(), &absent_drv("gone")).expect_err("no such file");
    assert!(matches!(err, DrvError::Read { .. }));
    assert!(err.to_string().contains("-gone.drv"), "{err}");
}

#[test]
fn a_floating_output_is_a_parse_error() {
    let store = FakeStore::new();
    let path = store.add_raw(
        "ca",
        br#"Derive([("out","","r:sha256","")],[],[],"x86_64-linux","/bin/sh",[],[("name","ca"),("out","")])"#,
    );
    let err = read(store.dir(), &path).expect_err("floating output");
    assert!(matches!(err, DrvError::Parse { .. }), "{err}");
}
