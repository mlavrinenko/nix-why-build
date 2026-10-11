//! Classifies every root: fetches and roots without a `pname` from their
//! `.drv` alone, the rest against their counterparts in the flake's nixpkgs
//! inputs, whose outputs (and each input's `hello`) the caches are asked about.

use std::collections::HashMap;
use std::path::Path;

use nix_derivation::StorePath;

use crate::cache::{Availability, Substituters};
use crate::counterparts::{self, Found, Input};
use crate::drv::{self, Drv};
use crate::modules::Labels;
use crate::reason::{self, Facts, Reason};
use crate::renamed;
use crate::roots::Split;
use crate::walk::Graph;
use crate::{Options, Progress};

/// What a root is looked up as: its `pname` and the package sets to try.
type Key = (String, Vec<String>);

/// What is known of the flake whose nixpkgs inputs roots are compared with.
#[derive(Clone, Copy, Debug)]
pub enum Inputs<'a> {
    /// No flake was given (a `.drv` path), so there are no inputs to read.
    NoFlake,
    /// The flake and its inputs, as `nix flake archive` listed them.
    Listed(&'a Labels),
    /// `nix flake archive` failed; its first error line says why.
    Unlisted(&'a str),
}

/// Each root's reason, keyed by root `.drv` path, and any warning: a failed
/// lookup leaves those roots unknown.
pub async fn classify_roots(
    options: &Options,
    graph: &Graph,
    split: &Split,
    flake: Inputs<'_>,
    substituters: &Substituters,
) -> (HashMap<String, Reason>, Vec<String>) {
    let store_dir = options.store_dir.as_path();
    let mut warnings = Vec::new();
    let roots: Vec<&Drv> = split
        .roots
        .iter()
        .filter_map(|path| graph.nodes.get(path)?.drv.as_ref())
        .collect();
    let system = graph
        .nodes
        .get(&graph.top)
        .and_then(|node| node.drv.as_ref())
        .map_or("x86_64-linux", |top| top.system.as_str());
    let keys = distinct_keys(&roots, system);
    let inputs = match flake {
        Inputs::Listed(labels) => labels.nixpkgs_inputs(),
        Inputs::NoFlake | Inputs::Unlisted(_) => Vec::new(),
    };
    let lookups: Vec<counterparts::Lookup> = keys
        .iter()
        .map(|(pname, variants)| counterparts::Lookup::new(pname, variants.clone()))
        .collect();
    announce(
        options,
        &roots,
        system,
        !inputs.is_empty() && !lookups.is_empty(),
    );
    let mut found = find(system, &inputs, &lookups, flake, &mut warnings).await;
    if let Ok(found) = &mut found {
        warnings.extend(renamed::find_renamed(system, &inputs, &lookups, found, options).await);
    }
    let facts = match &found {
        Ok(found) => facts(store_dir, found, substituters).await,
        Err(_) => Facts::default(),
    };
    let reasons = roots
        .iter()
        .map(|root| {
            let key = key_of(root, system);
            let at = key
                .as_ref()
                .ok()
                .and_then(|key| keys.iter().position(|seen| seen == key));
            let tried = at
                .and_then(|at| lookups.get(at))
                .map(|lookup| tried(&inputs, lookup))
                .unwrap_or_default();
            let lookup = match (&found, &key) {
                (Err(why), _) | (Ok(_), Err(why)) => reason::Lookup::Skipped(why),
                (Ok(found), Ok(_)) => reason::Lookup::Found(
                    at.and_then(|at| found.roots.get(at))
                        .map_or(&[], Vec::as_slice),
                    &tried,
                ),
            };
            (
                root.path.to_absolute_path(),
                reason::classify(store_dir, root, lookup, &facts),
            )
        })
        .collect();
    (reasons, warnings)
}

/// Tells the progress callback how many roots are about to be compared with
/// nixpkgs, when the comparison happens at all: roots a fetch, a local-only
/// build or a missing `pname` rules out are not compared.
fn announce(options: &Options, roots: &[&Drv], system: &str, comparing: bool) {
    if comparing {
        let compared = roots
            .iter()
            .filter(|root| key_of(root, system).is_ok())
            .count();
        options.notify(Progress::Comparing { roots: compared });
    }
}

async fn find(
    system: &str,
    inputs: &[Input],
    lookups: &[counterparts::Lookup],
    flake: Inputs<'_>,
    warnings: &mut Vec<String>,
) -> Result<Found, String> {
    if lookups.is_empty() {
        return Ok(Found::default());
    }
    if inputs.is_empty() {
        return Err(match flake {
            Inputs::Unlisted(why) => format!("the flake inputs could not be listed: {why}"),
            Inputs::NoFlake | Inputs::Listed(_) => {
                "no nixpkgs flake input to compare against".to_owned()
            }
        });
    }
    counterparts::find(system, inputs, lookups)
        .await
        .map_err(|err| {
            warnings.push(format!("nixpkgs counterparts unknown: {err}"));
            format!("looking it up in nixpkgs failed: {err}")
        })
}

/// The keys of every root worth a lookup, once each.
fn distinct_keys(roots: &[&Drv], system: &str) -> Vec<Key> {
    let mut keys: Vec<Key> = Vec::new();
    for key in roots.iter().filter_map(|root| key_of(root, system).ok()) {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

/// What to look `root` up as, or why it is not. A root built for
/// `i686-linux` on `x86_64-linux` is looked for in `pkgsi686Linux`, a CUDA
/// one in `pkgsCuda` too.
fn key_of(root: &Drv, system: &str) -> Result<Key, String> {
    let pname = root
        .pname
        .clone()
        .filter(|_| !root.fixed_output && !root.local_only)
        .ok_or_else(|| "it is a fetch, local-only or has no pname".to_owned())?;
    let base = if root.system == system {
        ""
    } else if root.system == "i686-linux" && system == "x86_64-linux" {
        "pkgsi686Linux"
    } else {
        return Err(format!(
            "it builds for {}, and only {system} is looked up",
            root.system
        ));
    };
    let mut variants = vec![base.to_owned()];
    if base.is_empty() && uses_cuda(root) {
        variants.push("pkgsCuda".to_owned());
    }
    Ok((pname, variants))
}

fn uses_cuda(root: &Drv) -> bool {
    root.inputs
        .keys()
        .any(|input| input.name().contains("cuda"))
}

/// `nixpkgs#llama-cpp`, `unstable#pkgsCuda.llama-cpp`, ... for every guess.
fn tried(inputs: &[Input], lookup: &counterparts::Lookup) -> Vec<String> {
    lookup
        .attrs
        .iter()
        .flat_map(|attr| {
            inputs.iter().flat_map(move |input| {
                lookup.variants.iter().map(move |variant| {
                    let set = if variant.is_empty() {
                        String::new()
                    } else {
                        format!("{variant}.")
                    };
                    format!("{}#{set}{}", input.label, attr.join("."))
                })
            })
        })
        .collect()
}

/// Asks the caches about every candidate's and every `hello`'s outputs: a
/// `.drv` counts as served when all of its outputs are.
async fn facts(store_dir: &Path, found: &Found, substituters: &Substituters) -> Facts {
    let hello: HashMap<String, String> = found
        .hello
        .iter()
        .filter_map(|entry| Some((entry.input.clone(), entry.drv.clone()?)))
        .collect();
    let outputs: HashMap<&String, Vec<StorePath>> = found
        .roots
        .iter()
        .flatten()
        .map(|candidate| &candidate.drv)
        .chain(hello.values())
        .filter_map(|drv| {
            let path = StorePath::from_absolute_path(drv.as_bytes()).ok()?;
            let read = drv::read(store_dir, &path).ok()?;
            Some((drv, read.outputs.into_values().collect()))
        })
        .collect();
    let all: Vec<StorePath> = outputs.values().flatten().cloned().collect();
    let answers = substituters.query(&all).await;
    let served = outputs
        .into_iter()
        .map(|(drv, outs)| {
            let served = outs
                .iter()
                .all(|out| matches!(answers.get(out), Some(Availability::Served { .. })));
            (drv.clone(), served)
        })
        .collect();
    Facts { served, hello }
}

#[cfg(test)]
#[path = "lookup_tests.rs"]
mod tests;
