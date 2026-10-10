//! What [`crate::explain`] returns. [`crate::human`] prints it.

use nix_compat::store_path::StorePath;
use serde::Serialize;

use crate::cache::Substituters;
use std::collections::HashMap;

use crate::modules::{Index, Labels};
use crate::reason::Reason;
use crate::requester::{Parents, Request, requests};
use crate::roots::{Chain, Split};
use crate::walk::Graph;

/// The answer: the roots of what gets built, the rest it drags in, and
/// what could not be decided.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The `.drv` the walk started from.
    pub toplevel: String,
    /// The substituters asked, in order.
    pub substituters: Vec<String>,
    /// Where the misses start: most named cascades first, then by name.
    pub roots: Vec<Root>,
    /// Every derivation no substituter serves, sorted by name.
    pub to_build: Vec<Package>,
    /// Derivations with no answer.
    pub unknown: Vec<Unknown>,
    /// What could not be worked out, said once each.
    pub warnings: Vec<String>,
}

/// One derivation that gets built.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Package {
    /// The derivation name, `pname-version` for a package.
    pub name: String,
    /// `pname`; config glue and most fetches have none.
    pub pname: Option<String>,
    /// `version`.
    pub version: Option<String>,
    /// The `.drv` path.
    pub drv: String,
}

/// A derivation built while every input it uses is served.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Root {
    /// The derivation name.
    pub name: String,
    /// `pname`, if any.
    pub pname: Option<String>,
    /// `version`, if any.
    pub version: Option<String>,
    /// The `.drv` path.
    pub drv: String,
    /// Whether it is a fetch (a fixed-output derivation).
    pub fixed_output: bool,
    /// What gets built because of it: nearest first, then by name.
    pub cascades: Vec<Cascade>,
    /// The options that pull it in, by option.
    pub requested_by: Vec<Requested>,
    /// Why no cache has it.
    pub reason: Reason,
}

/// One option that pulls a root in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Requested {
    /// The option, like `environment.systemPackages`, or `toplevel` when no
    /// glue node stands between the toplevel and the root.
    pub option: String,
    /// The glue derivation that option builds.
    pub via: String,
    /// Its `.drv` path.
    pub via_drv: String,
    /// What the option lists on the way to the root.
    pub item: String,
    /// That item's `.drv` path.
    pub item_drv: String,
    /// The module files that set the option, when the configuration could
    /// be indexed: relative to its flake, or `<input>:path` for an input's.
    pub files: Vec<String>,
}

/// A derivation built only because a root below it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Cascade {
    /// The derivation name.
    pub name: String,
    /// `pname`, if any.
    pub pname: Option<String>,
    /// The `.drv` path.
    pub drv: String,
    /// Names from this derivation down to the root, both included.
    pub chain: Vec<String>,
}

/// One derivation with no answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Unknown {
    /// The `.drv` path.
    pub drv: String,
    /// Why.
    pub reason: String,
}

impl Report {
    /// Collects the report from a finished walk and its split.
    #[must_use]
    pub fn new(graph: &Graph, split: &Split, substituters: &Substituters) -> Self {
        let parents = Parents::of(graph);
        let mut roots: Vec<Root> = split
            .roots
            .iter()
            .map(|path| {
                let mut root = root(graph, path, &cascades_of(split, path));
                root.requested_by = requests(graph, &parents, path)
                    .iter()
                    .map(|request| requested(graph, request))
                    .collect();
                root
            })
            .collect();
        roots.sort_by(|left, right| {
            named(&right.cascades)
                .cmp(&named(&left.cascades))
                .then_with(|| left.name.cmp(&right.name))
        });
        Self {
            toplevel: graph.top.to_absolute_path(),
            substituters: substituters.urls().into_iter().map(str::to_owned).collect(),
            roots,
            to_build: graph.to_build().into_iter().map(package).collect(),
            unknown: graph
                .unknown()
                .into_iter()
                .map(|(path, reason)| Unknown {
                    drv: path.to_absolute_path(),
                    reason: reason.to_owned(),
                })
                .collect(),
            warnings: Vec::new(),
        }
    }

    /// Sets each root's reason, by `.drv` path; roots not in `reasons` keep
    /// theirs.
    pub fn set_reasons(&mut self, reasons: &HashMap<String, Reason>) {
        for root in &mut self.roots {
            if let Some(reason) = reasons.get(&root.drv) {
                root.reason = reason.clone();
            }
        }
    }

    /// Resolves each request against the configuration's option index: the
    /// full option name, and the module files that set it, labelled.
    pub fn locate(&mut self, index: &Index, labels: &Labels) {
        for request in self
            .roots
            .iter_mut()
            .flat_map(|root| &mut root.requested_by)
        {
            let Some(located) = index.locate(
                &request.option,
                &request.via,
                &request.via_drv,
                &request.item_drv,
            ) else {
                continue;
            };
            request.option = located.option;
            request.files = located
                .files
                .iter()
                .map(|file| labels.label(file))
                .collect();
        }
    }
}

/// How many cascades of `cascades` have a `pname`.
pub(crate) fn named(cascades: &[Cascade]) -> usize {
    cascades
        .iter()
        .filter(|cascade| cascade.pname.is_some())
        .count()
}

/// Every cascade whose chains reach `root`, with the chain to it.
fn cascades_of<'a>(split: &'a Split, root: &StorePath) -> Vec<(&'a StorePath, &'a Chain)> {
    split
        .cascades
        .iter()
        .filter_map(|(cascade, chains)| {
            chains
                .iter()
                .find(|chain| &chain.root == root)
                .map(|chain| (cascade, chain))
        })
        .collect()
}

fn root(graph: &Graph, path: &StorePath, cascades: &[(&StorePath, &Chain)]) -> Root {
    let drv = graph.nodes.get(path).and_then(|node| node.drv.as_ref());
    let mut cascades: Vec<Cascade> = cascades
        .iter()
        .map(|(cascade, chain)| Cascade {
            name: graph.name(cascade).to_owned(),
            pname: graph
                .nodes
                .get(*cascade)
                .and_then(|node| node.drv.as_ref())
                .and_then(|drv| drv.pname.clone()),
            drv: cascade.to_absolute_path(),
            chain: chain
                .path
                .iter()
                .map(|step| graph.name(step).to_owned())
                .collect(),
        })
        .collect();
    cascades.sort_by(|left, right| {
        left.chain
            .len()
            .cmp(&right.chain.len())
            .then_with(|| left.name.cmp(&right.name))
    });
    Root {
        name: graph.name(path).to_owned(),
        pname: drv.and_then(|drv| drv.pname.clone()),
        version: drv.and_then(|drv| drv.version.clone()),
        drv: path.to_absolute_path(),
        fixed_output: drv.is_some_and(|drv| drv.fixed_output),
        cascades,
        requested_by: Vec::new(),
        reason: Reason::Unknown {
            because: "not classified".to_owned(),
        },
    }
}

fn requested(graph: &Graph, request: &Request) -> Requested {
    Requested {
        option: request.option.clone(),
        via: graph.name(&request.glue).to_owned(),
        via_drv: request.glue.to_absolute_path(),
        item: graph.name(&request.item).to_owned(),
        item_drv: request.item.to_absolute_path(),
        files: Vec::new(),
    }
}

fn package(drv: &crate::drv::Drv) -> Package {
    Package {
        name: drv.name.clone(),
        pname: drv.pname.clone(),
        version: drv.version.clone(),
        drv: drv.path.to_absolute_path(),
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
