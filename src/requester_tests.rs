use nix_derivation::StorePath;

use crate::fixture_tests::{FakeCache, FakeStore, spec};
use crate::walk::{Graph, walk};

use super::{Parents, requests};

async fn graph_of(store: &FakeStore, top: &StorePath) -> Graph {
    walk(store.dir(), top, &FakeCache::default()).await
}

/// `(option, glue name, item name)` for each request of `root`.
fn asked(graph: &Graph, root: &StorePath) -> Vec<(String, String, String)> {
    requests(graph, &Parents::of(graph), root)
        .into_iter()
        .map(|request| {
            (
                request.option,
                graph.name(&request.glue).to_owned(),
                graph.name(&request.item).to_owned(),
            )
        })
        .collect()
}

fn triple(option: &str, glue: &str, item: &str) -> (String, String, String) {
    (option.to_owned(), glue.to_owned(), item.to_owned())
}

#[tokio::test]
async fn a_package_in_system_path_is_wanted_by_system_packages() {
    let store = FakeStore::new();
    let llama = store.add(&spec("llama-cpp-0.5.0").env("pname", "llama-cpp"));
    let path = store.add(&spec("system-path").input(&llama, &["out"]));
    let top = store.add(&spec("nixos-system-tank").input(&path, &["out"]));
    let graph = graph_of(&store, &top).await;
    assert_eq!(
        asked(&graph, &llama),
        [triple(
            "environment.systemPackages",
            "system-path",
            "llama-cpp-0.5.0"
        )]
    );
}

#[tokio::test]
async fn every_glue_node_above_a_root_asks_for_it() {
    let store = FakeStore::new();
    let llama = store.add(&spec("llama-cpp-0.5.0"));
    let path = store.add(&spec("system-path").input(&llama, &["out"]));
    let unit = store.add(&spec("unit-llama-embed.service").input(&llama, &["out"]));
    let units = store.add(&spec("system-units").input(&unit, &["out"]));
    let etc = store.add(&spec("etc").input(&units, &["out"]));
    let top = store.add(
        &spec("nixos-system-tank")
            .input(&path, &["out"])
            .input(&etc, &["out"]),
    );
    let graph = graph_of(&store, &top).await;
    assert_eq!(
        asked(&graph, &llama),
        [
            triple(
                "environment.systemPackages",
                "system-path",
                "llama-cpp-0.5.0"
            ),
            triple(
                "systemd.services.llama-embed",
                "unit-llama-embed.service",
                "llama-cpp-0.5.0"
            ),
        ]
    );
}

#[tokio::test]
async fn the_item_is_what_the_glue_node_lists_not_the_root() {
    let store = FakeStore::new();
    let krate = store.add(&spec("arrayvec-0.7.8"));
    let vendor = store.add(&spec("cargo-vendor-dir").input(&krate, &["out"]));
    let kache = store.add(&spec("kache-1.0.0").input(&vendor, &["out"]));
    let path = store.add(&spec("system-path").input(&kache, &["out"]));
    let top = store.add(&spec("nixos-system-tank").input(&path, &["out"]));
    let graph = graph_of(&store, &top).await;
    assert_eq!(
        asked(&graph, &krate),
        [triple(
            "environment.systemPackages",
            "system-path",
            "kache-1.0.0"
        )]
    );
}

#[tokio::test]
async fn units_map_by_kind_and_user_units_by_their_parent() {
    let store = FakeStore::new();
    let tool = store.add(&spec("tool-1"));
    let timer = store.add(&spec("unit-backup.timer").input(&tool, &["out"]));
    let getty = store.add(&spec("unit-dbus.org.service").input(&tool, &["out"]));
    let user = store.add(&spec("unit-agent.socket").input(&tool, &["out"]));
    let system_units = store.add(
        &spec("system-units")
            .input(&timer, &["out"])
            .input(&getty, &["out"]),
    );
    let user_units = store.add(&spec("user-units").input(&user, &["out"]));
    let top = store.add(
        &spec("nixos-system-tank")
            .input(&system_units, &["out"])
            .input(&user_units, &["out"]),
    );
    let graph = graph_of(&store, &top).await;
    assert_eq!(
        asked(&graph, &tool),
        [
            triple(
                "systemd.services.\"dbus.org\"",
                "unit-dbus.org.service",
                "tool-1"
            ),
            triple("systemd.timers.backup", "unit-backup.timer", "tool-1"),
            triple("systemd.user.sockets.agent", "unit-agent.socket", "tool-1"),
        ]
    );
}

#[tokio::test]
async fn home_manager_and_etc_glue_have_their_options() {
    let store = FakeStore::new();
    let firefox = store.add(&spec("firefox-156.0.1"));
    let hm_path = store.add(&spec("home-manager-path").input(&firefox, &["out"]));
    let conf = store.add(&spec("fonts.conf").input(&firefox, &["out"]));
    let hm_files = store.add(&spec("home-manager-files").input(&conf, &["out"]));
    let etc = store.add(&spec("etc").input(&firefox, &["out"]));
    let users = store.add(&spec("user-environment").input(&firefox, &["out"]));
    let top = store.add(
        &spec("nixos-system-tank")
            .input(&hm_path, &["out"])
            .input(&hm_files, &["out"])
            .input(&etc, &["out"])
            .input(&users, &["out"]),
    );
    let graph = graph_of(&store, &top).await;
    assert_eq!(
        asked(&graph, &firefox),
        [
            triple("environment.etc", "etc", "firefox-156.0.1"),
            triple("home.file", "home-manager-files", "fonts.conf"),
            triple("home.packages", "home-manager-path", "firefox-156.0.1"),
            triple(
                "users.users.*.packages",
                "user-environment",
                "firefox-156.0.1"
            ),
        ]
    );
}

#[tokio::test]
async fn a_way_up_with_no_glue_says_toplevel() {
    let store = FakeStore::new();
    let locales = store.add(&spec("glibc-locales-2.42"));
    let top = store.add(&spec("nixos-system-tank").input(&locales, &["out"]));
    let graph = graph_of(&store, &top).await;
    assert_eq!(
        asked(&graph, &locales),
        [triple(
            "toplevel",
            "nixos-system-tank",
            "glibc-locales-2.42"
        )]
    );
}

#[tokio::test]
async fn the_toplevel_itself_is_asked_by_nobody() {
    let store = FakeStore::new();
    let top = store.add(&spec("hello-2.12"));
    let graph = graph_of(&store, &top).await;
    assert!(asked(&graph, &top).is_empty());
}

#[test]
fn every_unit_kind_maps_to_its_option() {
    let cases = [
        ("unit-a.service", false, Some("systemd.services.a")),
        ("unit-a.socket", false, Some("systemd.sockets.a")),
        ("unit-a.timer", false, Some("systemd.timers.a")),
        ("unit-a.path", false, Some("systemd.paths.a")),
        ("unit-a.mount", false, Some("systemd.mounts.a")),
        ("unit-a.automount", false, Some("systemd.automounts.a")),
        ("unit-a.slice", false, Some("systemd.slices.a")),
        ("unit-a.target", true, Some("systemd.user.targets.a")),
        ("unit-a.scope", false, None),
        ("unitless.service", false, None),
        ("unit-nodot", false, None),
    ];
    for (name, user, expected) in cases {
        assert_eq!(
            super::unit_option(name, user).as_deref(),
            expected,
            "{name}"
        );
    }
}

#[test]
fn attribute_names_are_quoted_only_when_they_must_be() {
    let cases = [
        ("sshd", "sshd"),
        ("_x", "_x"),
        ("nix-daemon", "nix-daemon"),
        ("a'b", "a'b"),
        ("1x", "\"1x\""),
        ("dbus.org", "\"dbus.org\""),
        ("", "\"\""),
    ];
    for (name, expected) in cases {
        assert_eq!(super::attr_name(name), expected, "{name}");
    }
}
