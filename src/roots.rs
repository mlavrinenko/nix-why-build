//! Split what gets built into roots and the cascades they cause.
//!
//! A root gets built while every input it uses is served, or because it is
//! local-only: the miss starts there. A cascade gets built only because some
//! input of it misses.

use std::collections::{HashMap, VecDeque};

use nix_derivation::StorePath;

use crate::walk::Graph;

/// One way down from a cascade to a root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chain {
    /// The root it ends at.
    pub root: StorePath,
    /// From the cascade (first) to the root (last).
    pub path: Vec<StorePath>,
}

/// Roots, and for each cascade the roots below it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Split {
    /// Every root, sorted by name.
    pub roots: Vec<StorePath>,
    /// Each cascade's roots, sorted by name, with a shortest chain to each.
    pub cascades: HashMap<StorePath, Vec<Chain>>,
}

/// Splits the derivations `graph` builds into roots and cascades.
#[must_use]
pub fn split(graph: &Graph) -> Split {
    let mut roots = Vec::new();
    let mut cascades = HashMap::new();
    for path in graph.builds() {
        if miss_inputs(graph, path).is_empty() {
            roots.push(path.clone());
        } else {
            cascades.insert(path.clone(), chains_from(graph, path));
        }
    }
    roots.sort_by(|left, right| by_name(graph, left, right));
    Split { roots, cascades }
}

/// The build inputs that make `path` a cascade. A local-only build is built
/// for its own sake: it is a root whatever its inputs, and makes no parent a
/// cascade, since Hydra builds and caches it like anything else.
fn miss_inputs<'a>(graph: &'a Graph, path: &StorePath) -> Vec<&'a StorePath> {
    if graph.local_only(path) {
        return Vec::new();
    }
    graph
        .build_inputs(path)
        .into_iter()
        .filter(|input| !graph.local_only(input))
        .collect()
}

fn by_name(graph: &Graph, left: &StorePath, right: &StorePath) -> std::cmp::Ordering {
    graph
        .name(left)
        .cmp(graph.name(right))
        .then_with(|| left.to_basename().cmp(&right.to_basename()))
}

/// Breadth-first from `start` over build inputs: the first time a root is
/// reached, the path there is a shortest chain to it.
fn chains_from(graph: &Graph, start: &StorePath) -> Vec<Chain> {
    let mut parent: HashMap<&StorePath, &StorePath> = HashMap::new();
    let mut queue = VecDeque::from([start]);
    let mut chains = Vec::new();
    while let Some(node) = queue.pop_front() {
        let inputs = miss_inputs(graph, node);
        if inputs.is_empty() {
            chains.push(Chain {
                root: node.clone(),
                path: walk_back(&parent, start, node),
            });
        }
        for input in inputs {
            if input != start && !parent.contains_key(input) {
                parent.insert(input, node);
                queue.push_back(input);
            }
        }
    }
    chains.sort_by(|left, right| by_name(graph, &left.root, &right.root));
    chains
}

fn walk_back(
    parent: &HashMap<&StorePath, &StorePath>,
    start: &StorePath,
    end: &StorePath,
) -> Vec<StorePath> {
    let mut path = vec![end.clone()];
    let mut node = end;
    while node != start {
        let Some(&up) = parent.get(node) else {
            break;
        };
        path.push(up.clone());
        node = up;
    }
    path.reverse();
    path
}

#[cfg(test)]
#[path = "roots_tests.rs"]
mod tests;
