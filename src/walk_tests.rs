use nix_derivation::StorePath;

use crate::fixture_tests::{DrvSpec, FakeCache as Fake, FakeStore, absent_drv, spec};

use crate::cache::Availability;

use super::{Graph, Query, Verdict, walk};

fn verdict<'a>(graph: &'a Graph, path: &StorePath) -> Option<&'a Verdict> {
    graph.nodes.get(path).map(|node| &node.verdict)
}

fn built(graph: &Graph) -> Vec<String> {
    graph
        .to_build()
        .iter()
        .map(|drv| drv.name.clone())
        .collect()
}

async fn run(store: &FakeStore, top: &StorePath, fake: &Fake) -> Graph {
    walk(store.dir(), top, fake).await
}

fn serving(served: Vec<StorePath>) -> Fake {
    Fake {
        served,
        ..Fake::default()
    }
}

fn failing(unknown: Vec<StorePath>) -> Fake {
    Fake {
        unknown,
        ..Fake::default()
    }
}

/// Writes `lib`, then a `top` wanting `wanted` of it; returns both.
fn over(store: &FakeStore, lib: &DrvSpec, wanted: &[&str]) -> (StorePath, StorePath) {
    let lib = store.add(lib);
    let top = store.add(&spec("top").input(&lib, wanted));
    (lib, top)
}

#[tokio::test]
async fn stops_at_a_served_input_without_reading_below_it() {
    let store = FakeStore::new();
    let below = absent_drv("never-read");
    let (lib, top) = over(&store, &spec("lib-1.0").input(&below, &["out"]), &["out"]);
    let fake = serving(vec![store.output(&lib, "out")]);
    let graph = run(&store, &top, &fake).await;
    assert_eq!(verdict(&graph, &top), Some(&Verdict::Build));
    assert_eq!(verdict(&graph, &lib), Some(&Verdict::Served));
    assert!(!graph.nodes.contains_key(&below));
    assert_eq!(built(&graph), ["top"]);
}

#[tokio::test]
async fn descends_through_every_miss() {
    let store = FakeStore::new();
    let leaf = store.add(&spec("leaf-1"));
    let mid = store.add(&spec("mid-1").input(&leaf, &["out"]));
    let top = store.add(&spec("top").input(&mid, &["out"]));
    let fake = Fake {
        served: vec![store.output(&leaf, "out")],
        ..Fake::default()
    };
    let graph = run(&store, &top, &fake).await;
    assert_eq!(built(&graph), ["mid-1", "top"]);
    assert_eq!(verdict(&graph, &leaf), Some(&Verdict::Served));
}

#[tokio::test]
async fn a_served_top_builds_nothing() {
    let store = FakeStore::new();
    let top = store.add(&spec("top").outputs(&["out", "doc"]));
    let fake = Fake {
        served: vec![store.output(&top, "out"), store.output(&top, "doc")],
        ..Fake::default()
    };
    let graph = run(&store, &top, &fake).await;
    assert_eq!(verdict(&graph, &top), Some(&Verdict::Served));
    assert!(built(&graph).is_empty());
}

#[tokio::test]
async fn the_top_needs_every_output() {
    let store = FakeStore::new();
    let top = store.add(&spec("top").outputs(&["out", "doc"]));
    let fake = Fake {
        served: vec![store.output(&top, "out")],
        ..Fake::default()
    };
    let graph = run(&store, &top, &fake).await;
    assert_eq!(verdict(&graph, &top), Some(&Verdict::Build));
}

#[tokio::test]
async fn only_the_outputs_a_parent_wants_decide() {
    let store = FakeStore::new();
    let (lib, top) = over(&store, &spec("lib-1.0").outputs(&["out", "dev"]), &["out"]);
    let fake = serving(vec![store.output(&lib, "out")]);
    let graph = run(&store, &top, &fake).await;
    assert_eq!(verdict(&graph, &lib), Some(&Verdict::Served));
    assert!(
        !fake
            .asked
            .lock()
            .expect("lock")
            .contains(&store.output(&lib, "dev"))
    );
}

#[tokio::test]
async fn a_second_parent_wanting_a_missing_output_reopens_a_served_input() {
    let store = FakeStore::new();
    let deep = store.add(&spec("deep-1"));
    let lib = store.add(
        &spec("lib-1.0")
            .outputs(&["out", "dev"])
            .input(&deep, &["out"]),
    );
    let late = store.add(&spec("late").input(&lib, &["dev"]));
    let top = store.add(&spec("top").input(&lib, &["out"]).input(&late, &["out"]));
    let fake = Fake {
        served: vec![store.output(&lib, "out"), store.output(&deep, "out")],
        ..Fake::default()
    };
    let graph = run(&store, &top, &fake).await;
    assert_eq!(verdict(&graph, &lib), Some(&Verdict::Build));
    assert_eq!(verdict(&graph, &deep), Some(&Verdict::Served));
    assert_eq!(built(&graph), ["late", "lib-1.0", "top"]);
}

#[tokio::test]
async fn an_unknown_answer_is_unknown_and_not_descended() {
    let store = FakeStore::new();
    let below = absent_drv("never-read");
    let (lib, top) = over(&store, &spec("lib-1.0").input(&below, &["out"]), &["out"]);
    let fake = failing(vec![store.output(&lib, "out")]);
    let graph = run(&store, &top, &fake).await;
    assert_eq!(
        verdict(&graph, &lib),
        Some(&Verdict::Unknown {
            reason: "down".to_owned()
        })
    );
    assert!(!graph.nodes.contains_key(&below));
}

#[tokio::test]
async fn a_missing_output_beats_an_unknown_one() {
    let store = FakeStore::new();
    let (lib, top) = over(
        &store,
        &spec("lib-1.0").outputs(&["out", "dev"]),
        &["out", "dev"],
    );
    let fake = failing(vec![store.output(&lib, "out")]);
    let graph = run(&store, &top, &fake).await;
    assert_eq!(verdict(&graph, &lib), Some(&Verdict::Build));
}

#[tokio::test]
async fn an_unreadable_drv_is_unknown_with_the_reason() {
    let store = FakeStore::new();
    let gone = absent_drv("gone");
    let top = store.add(&spec("top").input(&gone, &["out"]));
    let graph = run(&store, &top, &Fake::default()).await;
    assert!(matches!(
        verdict(&graph, &gone),
        Some(Verdict::Unknown { reason }) if reason.starts_with("read ")
    ));
    assert_eq!(graph.unknown().len(), 1);
}

#[tokio::test]
async fn a_shared_input_is_asked_about_once() {
    let store = FakeStore::new();
    let shared = store.add(&spec("shared-1"));
    let left = store.add(&spec("left").input(&shared, &["out"]));
    let right = store.add(&spec("right").input(&shared, &["out"]));
    let top = store.add(&spec("top").input(&left, &["out"]).input(&right, &["out"]));
    let fake = Fake::default();
    let graph = run(&store, &top, &fake).await;
    let shared_out = store.output(&shared, "out");
    let asked = fake.asked.lock().expect("lock");
    assert_eq!(asked.iter().filter(|path| **path == shared_out).count(), 1);
    assert_eq!(built(&graph), ["left", "right", "shared-1", "top"]);
}

#[tokio::test]
async fn a_local_only_derivation_is_built_unasked_and_its_inputs_decided_as_usual() {
    let store = FakeStore::new();
    let fetched = store.add(&spec("fetched-1"));
    let missing = store.add(&spec("missing-1"));
    let (glue, top) = over(
        &store,
        &spec("etc-bashrc")
            .env("allowSubstitutes", "")
            .input(&fetched, &["out"])
            .input(&missing, &["out"]),
        &["out"],
    );
    let fake = serving(vec![
        store.output(&glue, "out"),
        store.output(&fetched, "out"),
    ]);
    let graph = run(&store, &top, &fake).await;
    assert_eq!(built(&graph), ["etc-bashrc", "missing-1", "top"]);
    assert_eq!(verdict(&graph, &fetched), Some(&Verdict::Served));
    assert!(
        !fake
            .asked
            .lock()
            .expect("lock")
            .contains(&store.output(&glue, "out"))
    );
}

/// The fixture's cache answers `Query` here, where the walk lives: the
/// fixture is shared with `tests/cli.rs`, which cannot see this trait.
impl Query for Fake {
    async fn query(
        &self,
        paths: &[StorePath],
    ) -> std::collections::HashMap<StorePath, Availability> {
        self.asked
            .lock()
            .expect("lock")
            .extend(paths.iter().cloned());
        paths
            .iter()
            .map(|path| {
                let answer = if self.served.contains(path) {
                    Availability::Served {
                        substituter: "fake".to_owned(),
                    }
                } else if self.unknown.contains(path) {
                    Availability::Unknown {
                        reason: "down".to_owned(),
                    }
                } else {
                    Availability::Missing
                };
                (path.clone(), answer)
            })
            .collect()
    }
}
