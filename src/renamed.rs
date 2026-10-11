//! Roots whose `pname` no attribute guess finds, like `nss-cacert` at
//! `cacert`: looked up again under the attributes an index of each input says
//! carry that `pname`. Only a miss builds the index, and only once per input
//! revision: it is cached by the input's store path, which its `narHash` names.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::counterparts::{self, Found, Input, Lookup};
use crate::{Options, Progress};

/// The Nix function that indexes one input, called with `--impure`.
const RENAMED_EXPR: &str = include_str!("renamed.nix");

/// Each `pname` of an input's top level, to the attributes that carry it.
type Index = HashMap<String, Vec<String>>;

#[derive(Serialize)]
struct Query<'a> {
    system: &'a str,
    path: &'a str,
}

/// `$XDG_CACHE_HOME/nix-why-build`, else `$HOME/.cache/nix-why-build`; a
/// relative or empty directory counts as unset, as the XDG spec says.
#[must_use]
pub fn cache_dir(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let absolute = |dir: Option<OsString>| dir.map(PathBuf::from).filter(|dir| dir.is_absolute());
    let base = absolute(xdg).or_else(|| absolute(home).map(|home| home.join(".cache")))?;
    Some(base.join("nix-why-build"))
}

/// Looks the lookups that found nothing up again, under the attributes each
/// input's index gives their `pname`, and puts what they find in `found`.
/// Returns a warning per index that could not be built.
pub async fn find_renamed(
    system: &str,
    inputs: &[Input],
    lookups: &[Lookup],
    found: &mut Found,
    options: &Options,
) -> Vec<String> {
    let missed: Vec<usize> = (0..lookups.len())
        .filter(|&at| found.roots.get(at).is_some_and(Vec::is_empty))
        .collect();
    if missed.is_empty() {
        return Vec::new();
    }
    let mut warnings = Vec::new();
    let mut indexes = Vec::new();
    for input in inputs {
        match index(system, input, options).await {
            Ok(index) => indexes.push(index),
            Err(why) => warnings.push(format!("{}'s attributes unindexed: {why}", input.label)),
        }
    }
    let (at, again): (Vec<usize>, Vec<Lookup>) = missed
        .into_iter()
        .filter_map(|at| {
            let lookup = lookups.get(at)?;
            let attrs = renames(&indexes, lookup);
            (!attrs.is_empty()).then(|| {
                (
                    at,
                    Lookup {
                        attrs,
                        ..lookup.clone()
                    },
                )
            })
        })
        .unzip();
    if again.is_empty() {
        return warnings;
    }
    match counterparts::find(system, inputs, &again).await {
        Ok(refound) => {
            for (at, candidates) in at.into_iter().zip(refound.roots) {
                if let Some(slot) = found.roots.get_mut(at) {
                    *slot = candidates;
                }
            }
        }
        Err(why) => warnings.push(format!("renamed counterparts unknown: {why}")),
    }
    warnings
}

/// The attributes the indexes give `lookup`'s `pname`, once each, minus the
/// guesses it tried already.
fn renames(indexes: &[Index], lookup: &Lookup) -> Vec<Vec<String>> {
    let mut attrs: Vec<Vec<String>> = Vec::new();
    for attr in indexes
        .iter()
        .filter_map(|index| index.get(&lookup.pname))
        .flatten()
    {
        let path = vec![attr.clone()];
        if !lookup.attrs.contains(&path) && !attrs.contains(&path) {
            attrs.push(path);
        }
    }
    attrs
}

/// `input`'s index: read back from the cache, else built and cached.
async fn index(system: &str, input: &Input, options: &Options) -> Result<Index, String> {
    let file = options
        .cache_dir
        .as_deref()
        .and_then(|dir| cache_file(dir, system, input));
    if let Some(index) = file.as_deref().and_then(read_cached) {
        return Ok(index);
    }
    options.notify(Progress::IndexingAttributes {
        input: input.label.clone(),
    });
    let query = serde_json::to_string(&Query {
        system,
        path: &input.path,
    })
    .map_err(|err| err.to_string())?;
    let expression = format!(
        "({RENAMED_EXPR}) (builtins.fromJSON {})",
        counterparts::nix_string(&query)
    );
    let json =
        crate::eval::nix_with_stdin(&["eval", "--impure", "--json", "--file", "-"], &expression)
            .await
            .map_err(|err| crate::eval::first_error(&err.to_string()))?;
    let index = serde_json::from_str(&json).map_err(|err| err.to_string())?;
    if let Some(file) = file {
        write_cached(&file, &json);
    }
    Ok(index)
}

/// `<dir>/attrs/<hash>-source-x86_64-linux.json`: the store path's name
/// changes with the input's `narHash`.
fn cache_file(dir: &Path, system: &str, input: &Input) -> Option<PathBuf> {
    let name = Path::new(&input.path).file_name()?.to_str()?;
    Some(dir.join("attrs").join(format!("{name}-{system}.json")))
}

fn read_cached(file: &Path) -> Option<Index> {
    serde_json::from_slice(&std::fs::read(file).ok()?).ok()
}

/// Keeps `json` at `file`, or logs why not: the next run then builds the
/// index again. Written beside it and renamed, so a run beside this one reads
/// a whole index or none.
fn write_cached(file: &Path, json: &str) {
    let partial = file.with_extension(format!("{}.partial", std::process::id()));
    let written = file
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&partial, json))
        .and_then(|()| std::fs::rename(&partial, file));
    if let Err(err) = written {
        log::warn!("caching {}: {err}", file.display());
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::cache_dir;

    fn dir(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
        cache_dir(xdg.map(Into::into), home.map(Into::into))
    }

    #[test]
    fn the_cache_follows_xdg_then_home_and_ignores_relative_dirs() {
        let ours = |base: &str| Some(PathBuf::from(base).join("nix-why-build"));
        assert_eq!(dir(Some("/x"), Some("/h")), ours("/x"));
        assert_eq!(dir(None, Some("/h")), ours("/h/.cache"));
        assert_eq!(dir(Some(""), Some("/h")), ours("/h/.cache"));
        assert_eq!(dir(Some("x"), Some("/h")), ours("/h/.cache"));
        assert_eq!(dir(None, Some("h")), None);
        assert_eq!(dir(None, None), None);
    }
}
