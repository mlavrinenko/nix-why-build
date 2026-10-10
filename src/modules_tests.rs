use std::collections::BTreeMap;

use super::{INDEX_EXPR, Index, Labels, Listing, Located, Naming, Scope, Sources};

fn naming(file: &str, names: &[&str]) -> Naming {
    Naming {
        file: file.to_owned(),
        names: names.iter().map(|&name| name.to_owned()).collect(),
    }
}

fn nixos_scope() -> Scope {
    Scope {
        prefix: String::new(),
        glue: vec![],
        lists: BTreeMap::from([(
            "environment.systemPackages".to_owned(),
            vec![
                Listing {
                    file: "/src/tools.nix".to_owned(),
                    drvs: vec!["/nix/store/a-ripgrep.drv".to_owned()],
                },
                Listing {
                    file: "/src/llama.nix".to_owned(),
                    drvs: vec!["/nix/store/b-llama-cpp.drv".to_owned()],
                },
            ],
        )]),
        attrs: BTreeMap::from([
            (
                "systemd.services".to_owned(),
                vec![
                    naming("/src/llama.nix", &["llama-cpp-embed", "getty@"]),
                    naming("/src/gpu.nix", &["llama-cpp-embed"]),
                ],
            ),
            (
                "systemd.user.services".to_owned(),
                vec![naming("/src/kache.nix", &["kache"])],
            ),
        ]),
        sources: BTreeMap::from([(
            "environment.etc".to_owned(),
            Sources {
                entries: BTreeMap::from([(
                    "tmpfiles.d".to_owned(),
                    vec!["/nix/store/c-tmpfiles.d.drv".to_owned()],
                )]),
                definitions: vec![naming("/nixpkgs/tmpfiles.nix", &["tmpfiles.d"])],
            },
        )]),
    }
}

fn tank_scope() -> Scope {
    Scope {
        prefix: "home-manager.users.tank.".to_owned(),
        glue: vec![
            "/nix/store/d-home-manager-path.drv".to_owned(),
            "/nix/store/e-home-manager-files.drv".to_owned(),
        ],
        lists: BTreeMap::from([(
            "home.packages".to_owned(),
            vec![Listing {
                file: "/src/firefox.nix".to_owned(),
                drvs: vec!["/nix/store/f-firefox.drv".to_owned()],
            }],
        )]),
        attrs: BTreeMap::new(),
        sources: BTreeMap::from([
            (
                "home.file".to_owned(),
                Sources {
                    entries: BTreeMap::from([(
                        ".config/fontconfig/fonts.conf".to_owned(),
                        vec!["/nix/store/g-fonts.conf.drv".to_owned()],
                    )]),
                    definitions: vec![naming("/hm/xdg.nix", &[".config/fontconfig/fonts.conf"])],
                },
            ),
            (
                "xdg.configFile".to_owned(),
                Sources {
                    entries: BTreeMap::from([(
                        "fontconfig/fonts.conf".to_owned(),
                        vec!["/nix/store/g-fonts.conf.drv".to_owned()],
                    )]),
                    definitions: vec![naming("/src/fonts.nix", &["fontconfig/fonts.conf"])],
                },
            ),
        ]),
    }
}

fn index() -> Index {
    Index {
        scopes: vec![nixos_scope(), tank_scope()],
    }
}

fn located(option: &str, files: &[&str]) -> Located {
    Located {
        option: option.to_owned(),
        files: files.iter().map(|&file| file.to_owned()).collect(),
    }
}

#[test]
fn a_list_option_names_the_definitions_listing_the_item() {
    assert_eq!(
        index().locate(
            "environment.systemPackages",
            "system-path",
            "/nix/store/x-system-path.drv",
            "/nix/store/b-llama-cpp.drv"
        ),
        Some(located("environment.systemPackages", &["/src/llama.nix"]))
    );
}

#[test]
fn a_list_option_with_no_listing_definition_keeps_the_option() {
    assert_eq!(
        index().locate(
            "environment.systemPackages",
            "system-path",
            "/nix/store/x-system-path.drv",
            "/nix/store/z-other.drv"
        ),
        Some(located("environment.systemPackages", &[]))
    );
}

#[test]
fn home_manager_requests_resolve_to_the_user_whose_glue_it_is() {
    assert_eq!(
        index().locate(
            "home.packages",
            "home-manager-path",
            "/nix/store/d-home-manager-path.drv",
            "/nix/store/f-firefox.drv"
        ),
        Some(located(
            "home-manager.users.tank.home.packages",
            &["/src/firefox.nix"]
        ))
    );
}

#[test]
fn a_unit_names_every_definition_that_sets_it() {
    assert_eq!(
        index().locate(
            "systemd.services.llama-cpp-embed",
            "unit-llama-cpp-embed.service",
            "/nix/store/u.drv",
            "/nix/store/b-llama-cpp.drv"
        ),
        Some(located(
            "systemd.services.llama-cpp-embed",
            &["/src/llama.nix", "/src/gpu.nix"]
        ))
    );
}

#[test]
fn a_path_safe_unit_name_finds_its_real_name() {
    assert_eq!(
        index().locate(
            "systemd.services.getty-",
            "unit-getty-.service",
            "/nix/store/u.drv",
            "/nix/store/i.drv"
        ),
        Some(located("systemd.services.\"getty@\"", &["/src/llama.nix"]))
    );
}

#[test]
fn a_user_unit_looks_among_user_units() {
    assert_eq!(
        index().locate(
            "systemd.user.services.kache",
            "unit-kache.service",
            "/nix/store/u.drv",
            "/nix/store/i.drv"
        ),
        Some(located("systemd.user.services.kache", &["/src/kache.nix"]))
    );
}

#[test]
fn a_file_option_names_the_entry_whose_source_is_the_item() {
    assert_eq!(
        index().locate(
            "environment.etc",
            "etc",
            "/nix/store/x-etc.drv",
            "/nix/store/c-tmpfiles.d.drv"
        ),
        Some(located(
            "environment.etc.\"tmpfiles.d\"",
            &["/nixpkgs/tmpfiles.nix"]
        ))
    );
}

#[test]
fn an_xdg_entry_beats_the_home_file_it_becomes() {
    assert_eq!(
        index().locate(
            "home.file",
            "home-manager-files",
            "/nix/store/e-home-manager-files.drv",
            "/nix/store/g-fonts.conf.drv"
        ),
        Some(located(
            "home-manager.users.tank.xdg.configFile.\"fontconfig/fonts.conf\"",
            &["/src/fonts.nix"]
        ))
    );
}

#[test]
fn what_the_index_cannot_place_is_none() {
    let index = index();
    assert_eq!(
        index.locate(
            "toplevel",
            "nixos-system",
            "/nix/store/t.drv",
            "/nix/store/i.drv"
        ),
        None
    );
    assert_eq!(
        index.locate(
            "systemd.services.gone",
            "unit-gone.service",
            "/nix/store/u.drv",
            "/nix/store/i.drv"
        ),
        None
    );
    assert_eq!(
        index.locate(
            "environment.etc",
            "etc",
            "/nix/store/x-etc.drv",
            "/nix/store/z.drv"
        ),
        None
    );
    assert_eq!(
        index.locate(
            "systemd.services.x",
            "not-a-unit",
            "/nix/store/u.drv",
            "/nix/store/i.drv"
        ),
        None
    );
    assert_eq!(
        Index::default().locate(
            "environment.etc",
            "etc",
            "/nix/store/x.drv",
            "/nix/store/z.drv"
        ),
        None
    );
}

#[test]
fn labels_name_files_by_their_flake() {
    let json = r#"{"path": "/nix/store/s-source", "inputs": {
        "nixpkgs": {"path": "/nix/store/n-source", "inputs": {}},
        "home-manager": {"path": "/nix/store/h-source", "inputs": {"nixpkgs": {"path": "/nix/store/n-source"}, "lib": {"path": "/nix/store/l-source"}}}
    }}"#;
    let labels = Labels::parse(json).expect("valid archive json");
    assert_eq!(
        labels.label("/nix/store/s-source/apps/llama.nix"),
        "apps/llama.nix"
    );
    assert_eq!(
        labels.label("/nix/store/n-source/nixos/modules/x.nix"),
        "nixpkgs:nixos/modules/x.nix"
    );
    assert_eq!(
        labels.label("/nix/store/l-source/y.nix"),
        "home-manager/lib:y.nix"
    );
    assert_eq!(
        labels.label("/nix/store/s-sourcery/z.nix"),
        "/nix/store/s-sourcery/z.nix"
    );
    assert_eq!(
        labels.label("/etc/nixos/configuration.nix"),
        "/etc/nixos/configuration.nix"
    );
}

/// The index expression itself, run by the real `nix` against a
/// configuration built by hand from `builtins` alone.
#[tokio::test]
async fn the_index_expression_reads_definitions_and_sources() {
    let fake = r#"
      let
        hello = derivation { name = "hello"; builder = "/bin/sh"; system = "x86_64-linux"; };
      in {
        options = {
          environment.systemPackages.definitionsWithLocations = [
            { file = "/m/a.nix, via option flake.modules.nixos.a"; value = [ hello ]; }
            { file = "/m/b.nix"; value = [ "not a package" ]; }
          ];
          systemd.services.definitionsWithLocations = [ { file = "/m/a.nix"; value = { sshd = { }; }; } ];
          environment.etc.definitionsWithLocations = [ { file = "/m/c.nix"; value = { "hello.conf" = { }; }; } ];
        };
        config.environment.etc."hello.conf".source = hello;
      }"#;
    let expr = format!("({INDEX_EXPR}) ({fake})");
    let json = crate::eval::nix(&["eval", "--json", "--expr", &expr])
        .await
        .expect("nix evaluates the index");
    let index: Index = serde_json::from_str(&json).expect("an index");
    let [scope] = index.scopes.as_slice() else {
        panic!("one scope without home-manager: {index:?}");
    };
    let hello = scope
        .sources
        .get("environment.etc")
        .and_then(|etc| etc.entries.get("hello.conf"))
        .and_then(|drvs| drvs.first())
        .expect("the etc entry's source");
    assert!(hello.ends_with("-hello.drv"), "{hello}");
    assert_eq!(
        scope.lists.get("environment.systemPackages"),
        Some(&vec![
            Listing {
                file: "/m/a.nix".to_owned(),
                drvs: vec![hello.clone()]
            },
            Listing {
                file: "/m/b.nix".to_owned(),
                drvs: vec![]
            },
        ])
    );
    assert_eq!(
        scope.attrs.get("systemd.services"),
        Some(&vec![naming("/m/a.nix", &["sshd"])])
    );
    assert_eq!(scope.attrs.get("systemd.timers"), Some(&vec![]));
}

#[test]
fn nixpkgs_inputs_are_those_holding_all_packages() {
    let nixpkgs = tempfile::tempdir().expect("tempdir");
    let other = tempfile::tempdir().expect("tempdir");
    let nested = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(nested.path().join("pkgs/top-level")).expect("mkdir");
    std::fs::write(nested.path().join("pkgs/top-level/all-packages.nix"), "{ }").expect("write");
    let top = nixpkgs.path().join("pkgs/top-level");
    std::fs::create_dir_all(&top).expect("mkdir");
    std::fs::write(top.join("all-packages.nix"), "{ }").expect("write");
    let json = format!(
        r#"{{"path": "{}", "inputs": {{"unstable": {{"path": "{}"}}, "flake-parts": {{"path": "/nix/store/none"}}, "hm": {{"path": "/nix/store/hm", "inputs": {{"nixpkgs": {{"path": "{}"}}}}}}}}}}"#,
        other.path().display(),
        nixpkgs.path().display(),
        nested.path().display()
    );
    let labels = Labels::parse(&json).expect("valid");
    assert_eq!(
        labels.nixpkgs_inputs(),
        [crate::counterparts::Input {
            label: "unstable".to_owned(),
            path: nixpkgs.path().display().to_string()
        }]
    );
    let own = format!(r#"{{"path": "{}"}}"#, nixpkgs.path().display());
    assert_eq!(
        Labels::parse(&own).expect("valid").nixpkgs_inputs(),
        [crate::counterparts::Input {
            label: "self".to_owned(),
            path: nixpkgs.path().display().to_string()
        }]
    );
}
