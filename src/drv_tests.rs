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

#[test]
fn attrs_keep_structured_json_and_drop_output_paths() {
    let store = FakeStore::new();
    let json = r#"{"pname":"x","outputs":{"out":"y"}}"#;
    let lib = store.add(&spec("lib"));
    let path = store.add(
        &spec("x-1")
            .outputs(&["out", "dev"])
            .env("__json", json)
            .env("cmakeFlags", "-DX=1")
            .input(&lib, &["out"]),
    );
    let attrs = super::read_attrs(store.dir(), &path).expect("readable drv");
    assert_eq!(
        attrs.env,
        BTreeMap::from([
            ("__json".to_owned(), json.to_owned()),
            ("cmakeFlags".to_owned(), "-DX=1".to_owned()),
            ("name".to_owned(), "x-1".to_owned()),
        ])
    );
    assert_eq!(attrs.inputs, BTreeSet::from([lib]));
}

#[test]
fn flags_a_derivation_that_forbids_substitutes_as_nix_reads_it() {
    let store = FakeStore::new();
    let local = |spec: &crate::fixture_tests::DrvSpec| {
        read(store.dir(), &store.add(spec))
            .expect("readable drv")
            .local_only
    };
    assert!(local(&spec("etc-bashrc").env("allowSubstitutes", "")));
    assert!(local(&spec("zero").env("allowSubstitutes", "0")));
    assert!(local(
        &spec("units").env("__json", r#"{"allowSubstitutes":false}"#)
    ));
    assert!(!local(&spec("plain")));
    assert!(!local(&spec("one").env("allowSubstitutes", "1")));
    assert!(!local(
        &spec("on").env("__json", r#"{"allowSubstitutes":true}"#)
    ));
    assert!(!local(&spec("absent").env("__json", r#"{"pname":"x"}"#)));
}
