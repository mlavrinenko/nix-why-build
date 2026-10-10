//! What differs between a root and the nixpkgs derivation it stands for.
//!
//! Store hashes inside values are blanked first: a dependency built
//! differently changes every path that mentions it, and that is not an
//! attribute the user changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nix_derivation::StorePath;
use serde::Serialize;

use crate::drv::{Attrs, DrvError, read_attrs};

/// The differences, each list sorted by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Diff {
    /// Attributes only the root sets.
    pub added: Vec<String>,
    /// Attributes only the counterpart sets.
    pub removed: Vec<String>,
    /// Attributes both set, to different values.
    pub changed: Vec<String>,
    /// Input derivations, by name, only the root uses.
    pub inputs_added: Vec<String>,
    /// Input derivations, by name, only the counterpart uses.
    pub inputs_removed: Vec<String>,
    /// Inputs both use by name, built differently.
    pub inputs_changed: Vec<String>,
}

impl Diff {
    /// Reads both derivations from `store_dir` and diffs `ours` (the root)
    /// against `theirs` (the counterpart).
    ///
    /// # Errors
    ///
    /// The [`DrvError`] of whichever side cannot be read.
    pub fn read(store_dir: &Path, theirs: &StorePath, ours: &StorePath) -> Result<Self, DrvError> {
        Ok(Self::between(
            &read_attrs(store_dir, theirs)?,
            &read_attrs(store_dir, ours)?,
        ))
    }

    /// Diffs `ours` (the root) against `theirs` (the counterpart).
    #[must_use]
    pub fn between(theirs: &Attrs, ours: &Attrs) -> Self {
        let (theirs_attrs, ours_attrs) = (attributes(theirs), attributes(ours));
        let (theirs_inputs, ours_inputs) = (inputs(theirs), inputs(ours));
        Self {
            added: only_in(&ours_attrs, &theirs_attrs),
            removed: only_in(&theirs_attrs, &ours_attrs),
            changed: differing(&ours_attrs, &theirs_attrs),
            inputs_added: only_in(&ours_inputs, &theirs_inputs),
            inputs_removed: only_in(&theirs_inputs, &ours_inputs),
            inputs_changed: differing(&ours_inputs, &theirs_inputs),
        }
    }

    /// No difference at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    /// How many differences, all lists together.
    #[must_use]
    pub fn size(&self) -> usize {
        self.added.len()
            + self.removed.len()
            + self.changed.len()
            + self.inputs_added.len()
            + self.inputs_removed.len()
            + self.inputs_changed.len()
    }
}

/// Attributes with store hashes blanked; structured attrs key by key.
fn attributes(attrs: &Attrs) -> BTreeMap<String, String> {
    let mut flat: BTreeMap<String, String> = attrs
        .env
        .iter()
        .filter(|(key, _)| key.as_str() != "__json")
        .map(|(key, value)| (key.clone(), blank_hashes(value)))
        .collect();
    let structured = attrs.env.get("__json").and_then(|json| {
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json).ok()
    });
    for (key, value) in structured.into_iter().flatten() {
        flat.insert(key, blank_hashes(&value.to_string()));
    }
    flat
}

/// Input name to its `.drv` paths (one, unless a name repeats).
fn inputs(attrs: &Attrs) -> BTreeMap<String, BTreeSet<String>> {
    let mut inputs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for input in &attrs.inputs {
        inputs
            .entry(input.name().trim_end_matches(".drv").to_owned())
            .or_default()
            .insert(input.to_basename());
    }
    inputs
}

fn only_in<V>(left: &BTreeMap<String, V>, right: &BTreeMap<String, V>) -> Vec<String> {
    left.keys()
        .filter(|key| !right.contains_key(*key))
        .cloned()
        .collect()
}

fn differing<V: PartialEq>(left: &BTreeMap<String, V>, right: &BTreeMap<String, V>) -> Vec<String> {
    left.iter()
        .filter(|(key, value)| right.get(*key).is_some_and(|other| other != *value))
        .map(|(key, _)| key.clone())
        .collect()
}

/// Replaces the 32-character hash of every `/nix/store/<hash>-` with `…`.
fn blank_hashes(value: &str) -> String {
    const PREFIX: &str = "/nix/store/";
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(at) = rest.find(PREFIX) {
        let (before, after) = rest.split_at(at + PREFIX.len());
        out.push_str(before);
        let hash = after
            .get(..32)
            .filter(|hash| hash.bytes().all(|b| b.is_ascii_alphanumeric()));
        match (hash, after.get(32..33)) {
            (Some(_), Some("-")) => {
                out.push('…');
                rest = after.get(32..).unwrap_or_default();
            }
            _ => rest = after,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
