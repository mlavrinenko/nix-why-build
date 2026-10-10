use nix_derivation::StorePath;

use crate::fixture_tests::{FakeCache, FakeStore, spec};
use crate::walk::{Graph, walk};

use super::{Split, split};

async fn graph_of(store: &FakeStore, top: &StorePath, cache: &FakeCache) -> Graph {
    walk(store.dir(), top, cache, &|_| {}).await
}

fn names(graph: &Graph, paths: &[StorePath]) -> Vec<String> {
    paths
        .iter()
        .map(|path| graph.name(path).to_owned())
        .collect()
}

/// `(root, chain)` names for one cascade.
fn chains(graph: &Graph, split: &Split, cascade: &StorePath) -> Vec<(String, Vec<String>)> {
    split
        .cascades
        .get(cascade)
        .map(|chains| {
            chains
                .iter()
                .map(|chain| {
                    let root = names(graph, std::slice::from_ref(&chain.root)).join("");
                    (root, names(graph, &chain.path))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn a_build_over_served_inputs_is_a_root_and_its_parent_cascades() {
    let store = FakeStore::new();
    let lib = store.add(&spec("lib-1.0"));
    let app = store.add(&spec("app-1.0").input(&lib, &["out"]));
    let top = store.add(&spec("system").input(&app, &["out"]));
    let cache = FakeCache {
        served: vec![store.output(&lib, "out")],
        ..FakeCache::default()
    };
    let graph = graph_of(&store, &top, &cache).await;
    let split = split(&graph);
    assert_eq!(split.roots, std::slice::from_ref(&app));
    assert_eq!(
        chains(&graph, &split, &top),
        [(
            "app-1.0".to_owned(),
            vec!["system".to_owned(), "app-1.0".to_owned()]
        )]
    );
    assert!(!split.cascades.contains_key(&app));
}

#[tokio::test]
async fn a_cascade_names_every_root_below_it_in_name_order() {
    let store = FakeStore::new();
    let zeta = store.add(&spec("zeta-src.tar.xz").fixed());
    let alpha = store.add(&spec("alpha-1"));
    let left = store.add(&spec("left").input(&zeta, &["out"]));
    let right = store.add(&spec("right").input(&zeta, &["out"]).input(&alpha, &["out"]));
    let top = store.add(
        &spec("system")
            .input(&left, &["out"])
            .input(&right, &["out"]),
    );
    let graph = graph_of(&store, &top, &FakeCache::default()).await;
    let split = split(&graph);
    assert_eq!(split.roots, [alpha.clone(), zeta.clone()]);
    assert_eq!(
        chains(&graph, &split, &right),
        [
            (
                "alpha-1".to_owned(),
                vec!["right".to_owned(), "alpha-1".to_owned()]
            ),
            (
                "zeta-src.tar.xz".to_owned(),
                vec!["right".to_owned(), "zeta-src.tar.xz".to_owned()]
            ),
        ]
    );
    let top_chains = chains(&graph, &split, &top);
    assert_eq!(top_chains.len(), 2);
    assert!(
        top_chains.iter().all(|(_, chain)| chain.len() == 3),
        "{top_chains:?}"
    );
}

#[tokio::test]
async fn the_chain_is_a_shortest_one() {
    let store = FakeStore::new();
    let root = store.add(&spec("root-1"));
    let deep = store.add(&spec("deep").input(&root, &["out"]));
    let mid = store.add(&spec("mid").input(&deep, &["out"]));
    let top = store.add(&spec("system").input(&mid, &["out"]).input(&root, &["out"]));
    let graph = graph_of(&store, &top, &FakeCache::default()).await;
    let split = split(&graph);
    assert_eq!(
        chains(&graph, &split, &top),
        [(
            "root-1".to_owned(),
            vec!["system".to_owned(), "root-1".to_owned()]
        )]
    );
    assert_eq!(
        chains(&graph, &split, &mid),
        [(
            "root-1".to_owned(),
            vec!["mid".to_owned(), "deep".to_owned(), "root-1".to_owned()]
        )]
    );
}

#[tokio::test]
async fn an_unknown_input_does_not_make_a_cascade() {
    let store = FakeStore::new();
    let flaky = store.add(&spec("flaky-1"));
    let top = store.add(&spec("system").input(&flaky, &["out"]));
    let cache = FakeCache {
        unknown: vec![store.output(&flaky, "out")],
        ..FakeCache::default()
    };
    let graph = graph_of(&store, &top, &cache).await;
    let split = split(&graph);
    assert_eq!(split.roots, [top]);
    assert!(split.cascades.is_empty());
}

#[tokio::test]
async fn a_served_top_has_no_roots() {
    let store = FakeStore::new();
    let top = store.add(&spec("system"));
    let cache = FakeCache {
        served: vec![store.output(&top, "out")],
        ..FakeCache::default()
    };
    let graph = graph_of(&store, &top, &cache).await;
    let split = split(&graph);
    assert!(split.roots.is_empty());
    assert!(split.cascades.is_empty());
}

#[tokio::test]
async fn a_local_only_build_is_a_root_and_makes_no_parent_a_cascade() {
    let store = FakeStore::new();
    let lib = store.add(&spec("lib-1.0"));
    let wrapper = store.add(
        &spec("node-wrapper")
            .env("allowSubstitutes", "")
            .input(&lib, &["out"]),
    );
    let app = store.add(&spec("app-1.0").input(&wrapper, &["out"]));
    let top = store.add(&spec("system").input(&app, &["out"]));
    let graph = graph_of(&store, &top, &FakeCache::default()).await;
    let split = split(&graph);
    assert_eq!(
        names(&graph, &split.roots),
        ["app-1.0", "lib-1.0", "node-wrapper"]
    );
    assert_eq!(
        chains(&graph, &split, &top),
        [(
            "app-1.0".to_owned(),
            vec!["system".to_owned(), "app-1.0".to_owned()]
        )]
    );
}
