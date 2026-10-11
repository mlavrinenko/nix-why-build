use crate::modules::{Index, Located, Naming, Scope, load};

use super::{fixture, scratch_store, strings};

async fn index_of(configuration: &str) -> Index {
    scratch_store();
    let attr = format!(
        "path:{}#nixosConfigurations.{configuration}",
        fixture("config")
    );
    load(&attr).await.expect("nix evaluates the index")
}

fn naming(file: &str, names: &[&str]) -> Naming {
    Naming {
        file: file.to_owned(),
        names: strings(names),
    }
}

/// The one `.drv` that `option`'s `entry` has as its source, checked to be
/// `name`'s.
fn source_drv(scope: &Scope, option: &str, entry: &str, name: &str) -> String {
    let drvs = scope
        .sources
        .get(option)
        .and_then(|sources| sources.entries.get(entry))
        .expect("the entry");
    let [drv] = drvs.as_slice() else {
        panic!("one source for {option} {entry}: {drvs:?}");
    };
    assert!(
        drv.starts_with("/nix/store/") && drv.ends_with(&format!("-{name}.drv")),
        "{drv}"
    );
    drv.clone()
}

#[tokio::test]
async fn the_nixos_scope_indexes_packages_units_and_sources() {
    let index = index_of("mypc").await;
    let [nixos, _alice] = index.scopes.as_slice() else {
        panic!("the system, then its one home-manager user: {index:?}");
    };
    assert_eq!((nixos.prefix.as_str(), nixos.glue.len()), ("", 0));
    let hello = source_drv(nixos, "environment.etc", "hello.conf", "hello-2.12");
    let listing = |file: &str, drvs: &[&str]| crate::modules::Listing {
        file: file.to_owned(),
        drvs: strings(drvs),
    };
    assert_eq!(
        nixos.lists.get("environment.systemPackages"),
        Some(&vec![
            listing("/m/a.nix", &[&hello]),
            listing("/m/b.nix", &[]),
            listing("/m/c.nix", &[]),
        ]),
        "`, via option` is cut off; what is no package, or does not evaluate, lists none"
    );
    assert_eq!(nixos.lists.get("home.packages"), Some(&vec![]));
    assert_eq!(
        nixos.attrs.get("systemd.services"),
        Some(&vec![
            naming("/m/f.nix", &["nix-daemon", "sshd"]),
            naming("/m/g.nix", &["sshd"]),
        ])
    );
    assert_eq!(
        nixos.attrs.get("systemd.user.timers"),
        Some(&vec![naming("/m/h.nix", &["backup"])])
    );
    assert_eq!(nixos.attrs.get("systemd.timers"), Some(&vec![]));
    assert_eq!(nixos.attrs.len(), 16, "eight unit kinds, system and user");
    let etc = nixos
        .sources
        .get("environment.etc")
        .expect("environment.etc");
    assert_eq!(
        etc.definitions,
        [
            naming("/m/d.nix", &["hello.conf", "plain"]),
            naming("/m/e.nix", &["htop.conf"])
        ]
    );
    let htop = source_drv(nixos, "environment.etc", "htop.conf", "htop-3.3");
    assert_eq!(
        etc.entries.values().cloned().collect::<Vec<_>>(),
        [vec![hello], vec![htop], vec![]],
        "a derivation and an interpolated string are sources, `text` is none"
    );
    for option in ["home.file", "xdg.configFile", "xdg.dataFile"] {
        let none = nixos.sources.get(option).expect(option);
        assert!(
            none.entries.is_empty() && none.definitions.is_empty(),
            "{option}"
        );
    }
}

#[tokio::test]
async fn a_home_manager_user_gets_a_scope_with_its_own_glue() {
    let index = index_of("mypc").await;
    let [_nixos, alice] = index.scopes.as_slice() else {
        panic!("the system, then its one home-manager user: {index:?}");
    };
    assert_eq!(alice.prefix, "home-manager.users.alice.");
    let [path, files] = alice.glue.as_slice() else {
        panic!("home.path and home-files: {:?}", alice.glue);
    };
    assert!(path.ends_with("-home-manager-path.drv"), "{path}");
    assert!(files.ends_with("-home-manager-files.drv"), "{files}");
    let hello = source_drv(alice, "home.file", ".config/hello", "hello-2.12");
    let fonts = source_drv(alice, "xdg.configFile", "fonts/fonts.conf", "fonts-1");
    let htop = alice
        .lists
        .get("home.packages")
        .and_then(|listings| listings.first())
        .expect("a listing");
    assert_eq!(htop.file, "/m/i.nix");
    assert!(
        matches!(htop.drvs.as_slice(), [drv] if drv.ends_with("-htop-3.3.drv")),
        "{htop:?}"
    );
    assert_eq!(
        alice.lists.len(),
        1,
        "the user's list options are home.packages only"
    );
    assert_eq!(
        alice.attrs.get("systemd.user.services"),
        Some(&vec![naming("/m/l.nix", &["syncthing"])])
    );
    assert_eq!(alice.attrs.len(), 8, "eight user unit kinds");
    assert_eq!(
        alice
            .sources
            .get("home.file")
            .map(|sources| &sources.definitions),
        Some(&vec![naming("/m/j.nix", &[".config/hello"])])
    );
    assert_eq!(
        alice
            .sources
            .get("xdg.configFile")
            .map(|sources| sources.entries.clone()),
        Some([("fonts/fonts.conf".to_owned(), vec![fonts])].into())
    );
    assert_eq!(
        alice
            .sources
            .get("home.file")
            .map(|sources| sources.entries.clone()),
        Some([(".config/hello".to_owned(), vec![hello])].into())
    );
}

#[tokio::test]
async fn a_configuration_without_home_manager_or_options_has_one_empty_scope() {
    let index = index_of("bare").await;
    let [scope] = index.scopes.as_slice() else {
        panic!("one scope: {index:?}");
    };
    assert!(scope.glue.is_empty());
    assert_eq!(scope.lists.len(), 2);
    assert!(scope.lists.values().all(Vec::is_empty));
    assert_eq!(scope.attrs.len(), 16);
    assert!(scope.attrs.values().all(Vec::is_empty));
    assert_eq!(scope.sources.len(), 4);
    assert!(
        scope
            .sources
            .values()
            .all(|sources| sources.entries.is_empty() && sources.definitions.is_empty())
    );
}

fn located(option: &str, files: &[&str]) -> Located {
    Located {
        option: option.to_owned(),
        files: strings(files),
    }
}

#[tokio::test]
async fn the_index_resolves_requests_to_options_and_module_files() {
    let index = index_of("mypc").await;
    let [nixos, alice] = index.scopes.as_slice() else {
        panic!("the system, then its one home-manager user: {index:?}");
    };
    let hello = source_drv(nixos, "environment.etc", "hello.conf", "hello-2.12");
    let htop = source_drv(nixos, "environment.etc", "htop.conf", "htop-3.3");
    let [home_path, home_files] = alice.glue.as_slice() else {
        panic!("home.path and home-files: {:?}", alice.glue);
    };
    let none = "/nix/store/none.drv";
    let user = "home-manager.users.alice";
    let cases = [
        (
            index.locate("environment.systemPackages", "nixos-system", none, &hello),
            located("environment.systemPackages", &["/m/a.nix"]),
        ),
        (
            index.locate("environment.etc", "etc", none, &htop),
            located("environment.etc.\"htop.conf\"", &["/m/e.nix"]),
        ),
        (
            index.locate("systemd.services.sshd", "unit-sshd.service", none, none),
            located("systemd.services.sshd", &["/m/f.nix", "/m/g.nix"]),
        ),
        (
            index.locate("home.packages", "home-manager-path", home_path, &htop),
            located(&format!("{user}.home.packages"), &["/m/i.nix"]),
        ),
        (
            index.locate("home.file", "home-manager-files", home_files, &hello),
            located(
                &format!("{user}.home.file.\".config/hello\""),
                &["/m/j.nix"],
            ),
        ),
        (
            index.locate(
                "systemd.user.services.syncthing",
                "unit-syncthing.service",
                home_files,
                none,
            ),
            located(
                &format!("{user}.systemd.user.services.syncthing"),
                &["/m/l.nix"],
            ),
        ),
    ];
    for (found, expected) in cases {
        assert_eq!(found, Some(expected));
    }
}
