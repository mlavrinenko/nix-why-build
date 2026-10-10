//! Walk the derivation graph from a toplevel down to what substituters serve.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::path::Path;

use nix_derivation::StorePath;

use crate::cache::Availability;
use crate::drv::Drv;

/// Answers whether store paths are served; [`crate::cache::Substituters`] is one.
pub trait Query {
    /// Availability of every path in `paths`.
    fn query(
        &self,
        paths: &[StorePath],
    ) -> impl Future<Output = HashMap<StorePath, Availability>> + Send;
}

/// What a build on an empty store does with one derivation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every wanted output is substituted.
    Served,
    /// Some wanted output is in no cache, so it gets built.
    Build,
    /// No answer: unreadable `.drv`, or a cache that could not answer.
    Unknown {
        /// Why.
        reason: String,
    },
}

/// One derivation the walk reached.
#[derive(Clone, Debug)]
pub struct Node {
    /// The parsed derivation, `None` when it could not be read.
    pub drv: Option<Drv>,
    /// The outputs some parent wants.
    pub wanted: BTreeSet<String>,
    /// The decision for those outputs.
    pub verdict: Verdict,
}

/// Every derivation the walk reached, keyed by `.drv` path.
#[derive(Clone, Debug)]
pub struct Graph {
    /// Where the walk started.
    pub top: StorePath,
    /// Reached derivations.
    pub nodes: HashMap<StorePath, Node>,
}

impl Graph {
    /// Every derivation that gets built, in no particular order.
    pub fn builds(&self) -> impl Iterator<Item = &StorePath> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.verdict == Verdict::Build)
            .map(|(path, _)| path)
    }

    /// The inputs of `path` that get built themselves, in path order.
    #[must_use]
    pub fn build_inputs(&self, path: &StorePath) -> Vec<&StorePath> {
        self.nodes
            .get(path)
            .and_then(|node| node.drv.as_ref())
            .map(|drv| {
                drv.inputs
                    .keys()
                    .filter(|input| {
                        self.nodes
                            .get(*input)
                            .is_some_and(|node| node.verdict == Verdict::Build)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether nix builds `path` without asking a cache, as it does any
    /// derivation with `allowSubstitutes = false`.
    #[must_use]
    pub fn local_only(&self, path: &StorePath) -> bool {
        local_only(&self.nodes, path)
    }

    /// The derivation's name, or the `.drv` file's when it was not read.
    #[must_use]
    pub fn name<'a>(&'a self, path: &'a StorePath) -> &'a str {
        self.nodes
            .get(path)
            .and_then(|node| node.drv.as_ref())
            .map_or_else(
                || path.name().trim_end_matches(".drv"),
                |drv| drv.name.as_str(),
            )
    }

    /// Derivations that get built, sorted by name.
    #[must_use]
    pub fn to_build(&self) -> Vec<&Drv> {
        let mut drvs: Vec<&Drv> = self
            .nodes
            .values()
            .filter(|node| node.verdict == Verdict::Build)
            .filter_map(|node| node.drv.as_ref())
            .collect();
        drvs.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.path.to_basename().cmp(&right.path.to_basename()))
        });
        drvs
    }

    /// Derivations with no answer, and why, sorted by path.
    #[must_use]
    pub fn unknown(&self) -> Vec<(&StorePath, &str)> {
        let mut unknown: Vec<(&StorePath, &str)> = self
            .nodes
            .iter()
            .filter_map(|(path, node)| match &node.verdict {
                Verdict::Unknown { reason } => Some((path, reason.as_str())),
                Verdict::Served | Verdict::Build => None,
            })
            .collect();
        unknown.sort_by_key(|(path, _)| path.to_basename());
        unknown
    }
}

/// Walks from `top`, reading `.drv` files from `store_dir`, one batched
/// query per level. It descends only into derivations that get built: the
/// cut nix makes on an empty store.
pub async fn walk<Q: Query>(store_dir: &Path, top: &StorePath, query: &Q) -> Graph {
    let mut state = State {
        nodes: HashMap::new(),
        answers: HashMap::new(),
        expanded: HashSet::new(),
    };
    let mut level: HashMap<StorePath, BTreeSet<String>> =
        HashMap::from([(top.clone(), BTreeSet::new())]);
    let mut first = true;
    while !level.is_empty() {
        state.read_new(store_dir, &level).await;
        if first {
            state.want_every_output(top);
            first = false;
        }
        let level_paths = state.merge_wanted(level);
        let ask = state.unasked_outputs(&level_paths);
        let started = std::time::Instant::now();
        if !ask.is_empty() {
            state.answers.extend(query.query(&ask).await);
        }
        log::info!(
            "level: {} drvs, {} outputs asked in {:?}",
            level_paths.len(),
            ask.len(),
            started.elapsed()
        );
        level = state.decide(&level_paths);
    }
    Graph {
        top: top.clone(),
        nodes: state.nodes,
    }
}

fn local_only(nodes: &HashMap<StorePath, Node>, path: &StorePath) -> bool {
    nodes
        .get(path)
        .and_then(|node| node.drv.as_ref())
        .is_some_and(|drv| drv.local_only)
}

struct State {
    nodes: HashMap<StorePath, Node>,
    answers: HashMap<StorePath, Availability>,
    expanded: HashSet<StorePath>,
}

impl State {
    /// Reads every `.drv` of the level not read before, off the async threads.
    async fn read_new(&mut self, store_dir: &Path, level: &HashMap<StorePath, BTreeSet<String>>) {
        let fresh: Vec<StorePath> = level
            .keys()
            .filter(|path| !self.nodes.contains_key(*path))
            .cloned()
            .collect();
        let dir = store_dir.to_path_buf();
        let read = tokio::task::spawn_blocking(move || {
            fresh
                .into_iter()
                .map(|path| {
                    let drv = crate::drv::read(&dir, &path);
                    (path, drv)
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        for (path, drv) in read {
            let node = match drv {
                Ok(drv) => Node {
                    drv: Some(drv),
                    wanted: BTreeSet::new(),
                    verdict: Verdict::Served,
                },
                Err(err) => Node {
                    drv: None,
                    wanted: BTreeSet::new(),
                    verdict: Verdict::Unknown {
                        reason: err.to_string(),
                    },
                },
            };
            self.nodes.insert(path, node);
        }
    }

    fn want_every_output(&mut self, top: &StorePath) {
        if let Some(node) = self.nodes.get_mut(top)
            && let Some(drv) = &node.drv
        {
            node.wanted = drv.outputs.keys().cloned().collect();
        }
    }

    /// Folds the level's wants into its nodes; returns the readable ones.
    /// Re-deciding a node reached again is cheap, since answers are kept.
    fn merge_wanted(&mut self, level: HashMap<StorePath, BTreeSet<String>>) -> Vec<StorePath> {
        let mut readable = Vec::new();
        for (path, wanted) in level {
            let Some(node) = self.nodes.get_mut(&path) else {
                continue;
            };
            if node.drv.is_none() {
                continue;
            }
            node.wanted.extend(wanted);
            readable.push(path);
        }
        readable
    }

    fn unasked_outputs(&self, level: &[StorePath]) -> Vec<StorePath> {
        let mut ask: HashSet<StorePath> = HashSet::new();
        for path in level.iter().filter(|path| !local_only(&self.nodes, path)) {
            for out in self.wanted_outputs(path) {
                if !self.answers.contains_key(out) {
                    ask.insert(out.clone());
                }
            }
        }
        ask.into_iter().collect()
    }

    fn wanted_outputs(&self, path: &StorePath) -> Vec<&StorePath> {
        self.nodes
            .get(path)
            .and_then(|node| {
                let drv = node.drv.as_ref()?;
                Some(
                    node.wanted
                        .iter()
                        .filter_map(|name| drv.outputs.get(name))
                        .collect(),
                )
            })
            .unwrap_or_default()
    }

    /// Decides each node of the level; returns the next level, the inputs of
    /// every node that turned into a build.
    fn decide(&mut self, level: &[StorePath]) -> HashMap<StorePath, BTreeSet<String>> {
        let mut next: HashMap<StorePath, BTreeSet<String>> = HashMap::new();
        for path in level {
            let verdict = self.verdict_of(path);
            let Some(node) = self.nodes.get_mut(path) else {
                continue;
            };
            node.verdict = verdict;
            if node.verdict != Verdict::Build || !self.expanded.insert(path.clone()) {
                continue;
            }
            for (input, wanted) in node.drv.iter().flat_map(|drv| &drv.inputs) {
                next.entry(input.clone())
                    .or_default()
                    .extend(wanted.iter().cloned());
            }
        }
        next
    }

    fn verdict_of(&self, path: &StorePath) -> Verdict {
        if local_only(&self.nodes, path) {
            return Verdict::Build;
        }
        let mut unknown = None;
        for out in self.wanted_outputs(path) {
            match self.answers.get(out) {
                Some(Availability::Missing) | None => return Verdict::Build,
                Some(Availability::Unknown { reason }) => {
                    unknown.get_or_insert_with(|| reason.clone());
                }
                Some(Availability::Served { .. }) => {}
            }
        }
        unknown.map_or(Verdict::Served, |reason| Verdict::Unknown { reason })
    }
}

#[cfg(test)]
#[path = "walk_tests.rs"]
mod tests;
