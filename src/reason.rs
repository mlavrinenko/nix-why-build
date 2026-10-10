//! Why a root misses: one category, with the evidence for it.

use std::collections::HashMap;
use std::path::Path;

use nix_derivation::StorePath;
use serde::Serialize;

use crate::counterparts::Candidate;
use crate::diff::Diff;
use crate::drv::{Drv, read_attrs};

/// Why a root is in no cache.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[non_exhaustive]
#[serde(tag = "category", rename_all = "kebab-case")]
pub enum Reason {
    /// `allowSubstitutes = false`: nix builds it without asking a cache.
    LocalOnly,
    /// A fixed-output fetch no cache has: the build downloads it.
    Fetch {
        /// Where it downloads from.
        urls: Vec<String>,
    },
    /// It differs from what nixpkgs builds under the same attribute.
    Overridden {
        /// The counterpart, like `unstable#pkgsCuda.llama-cpp`.
        against: String,
        /// Whether a cache has the counterpart.
        against_cached: bool,
        /// What differs.
        diff: Diff,
    },
    /// nixpkgs builds exactly this, and it is unfree: Hydra skips it.
    Unfree {
        /// The counterpart.
        against: String,
    },
    /// nixpkgs builds exactly this, and its meta keeps Hydra off it.
    NotBuiltByHydra {
        /// The counterpart.
        against: String,
        /// `meta.broken`, or the `meta.hydraPlatforms` it has.
        because: String,
    },
    /// nixpkgs builds exactly this, but the cache has not caught up with
    /// that revision: its `hello` is missing too.
    NewerThanCache {
        /// The counterpart.
        against: String,
    },
    /// No nixpkgs input has an attribute under the names tried: a custom
    /// package, or one nixpkgs names differently.
    NoCounterpart {
        /// The attributes looked for.
        tried: Vec<String>,
    },
    /// None of the above could be shown.
    Unknown {
        /// What was found instead.
        because: String,
    },
}

impl Reason {
    /// The category alone, as `--json` spells it.
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            Self::LocalOnly => "local-only",
            Self::Fetch { .. } => "fetch",
            Self::Overridden { .. } => "overridden",
            Self::Unfree { .. } => "unfree",
            Self::NotBuiltByHydra { .. } => "not-built-by-hydra",
            Self::NewerThanCache { .. } => "newer-than-cache",
            Self::NoCounterpart { .. } => "no-counterpart",
            Self::Unknown { .. } => "unknown",
        }
    }

    /// The evidence, one line.
    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::LocalOnly => {
                "allowSubstitutes = false: nix builds it even when a cache has it".to_owned()
            }
            Self::Fetch { urls } => urls.first().map_or_else(
                || "no cache has the source".to_owned(),
                |url| format!("no cache has {url}"),
            ),
            Self::Overridden {
                against,
                against_cached,
                diff,
            } => {
                let cached = if *against_cached {
                    "cached"
                } else {
                    "not cached either"
                };
                format!("vs {against} ({cached}): {}", describe(diff))
            }
            Self::Unfree { against } => format!("Hydra skips unfree {against}"),
            Self::NotBuiltByHydra { against, because } => format!("{against} has {because}"),
            Self::NewerThanCache { against } => {
                format!("{against} is unchanged, but that nixpkgs' hello is not cached either")
            }
            Self::NoCounterpart { tried } => format!(
                "no nixpkgs attribute matches its pname (tried {})",
                capped(tried, 3)
            ),
            Self::Unknown { because } => because.clone(),
        }
    }
}

/// `postPatch added; cmakeFlags changed; input blas-3 added`.
fn describe(diff: &Diff) -> String {
    let parts = [
        (&diff.added, "added"),
        (&diff.removed, "removed"),
        (&diff.changed, "changed"),
    ]
    .into_iter()
    .filter(|(names, _)| !names.is_empty())
    .map(|(names, what)| format!("{} {what}", capped(names, 3)))
    .chain(
        [
            (&diff.inputs_added, "added"),
            (&diff.inputs_removed, "removed"),
            (&diff.inputs_changed, "built differently"),
        ]
        .into_iter()
        .filter(|(names, _)| !names.is_empty())
        .map(|(names, what)| {
            let noun = if names.len() == 1 { "input" } else { "inputs" };
            format!("{noun} {} {what}", capped(names, 3))
        }),
    )
    .collect::<Vec<_>>();
    if parts.is_empty() {
        "no attribute differs".to_owned()
    } else {
        parts.join("; ")
    }
}

/// `a, b, c +2`.
fn capped(names: &[String], cap: usize) -> String {
    let shown = names.get(..cap).unwrap_or(names);
    let mut text = shown.join(", ");
    if names.len() > shown.len() {
        text.push_str(&format!(" +{}", names.len() - shown.len()));
    }
    text
}

/// What the caches said about counterparts and each input's `hello`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// `.drv` path to whether every output of it is served.
    pub served: HashMap<String, bool>,
    /// Input label to its `hello`'s `.drv`.
    pub hello: HashMap<String, String>,
}

impl Facts {
    fn served(&self, drv: &str) -> Option<bool> {
        self.served.get(drv).copied()
    }

    fn hello_served(&self, input: &str) -> Option<bool> {
        self.hello.get(input).and_then(|drv| self.served(drv))
    }
}

/// What looking a root up in nixpkgs gave.
#[derive(Clone, Copy, Debug)]
pub enum Lookup<'a> {
    /// Every candidate found, and the attributes tried.
    Found(&'a [Candidate], &'a [String]),
    /// It could not be looked up, and why.
    Skipped(&'a str),
}

/// Classifies `root`, reading `.drv` files from `store_dir`.
#[must_use]
pub fn classify(store_dir: &Path, root: &Drv, lookup: Lookup<'_>, facts: &Facts) -> Reason {
    if root.local_only {
        return Reason::LocalOnly;
    }
    if root.fixed_output {
        return Reason::Fetch {
            urls: read_attrs(store_dir, &root.path)
                .map(|attrs| fetch_urls(&attrs.env))
                .unwrap_or_default(),
        };
    }
    if root.pname.is_none() {
        return unknown("it has no pname to look up in nixpkgs");
    }
    let (candidates, tried) = match lookup {
        Lookup::Skipped(why) => return unknown(why),
        Lookup::Found(candidates, tried) => (candidates, tried),
    };
    if candidates.is_empty() {
        return Reason::NoCounterpart {
            tried: tried.to_vec(),
        };
    }
    let path = root.path.to_absolute_path();
    if let Some(same) = candidates.iter().find(|candidate| candidate.drv == path) {
        return identical(root, same, facts);
    }
    overridden(store_dir, root, candidates, facts)
}

fn unknown(because: &str) -> Reason {
    Reason::Unknown {
        because: because.to_owned(),
    }
}

/// nixpkgs builds this very derivation: something about it keeps it out.
fn identical(root: &Drv, same: &Candidate, facts: &Facts) -> Reason {
    let against = same.label();
    if same.unfree {
        return Reason::Unfree { against };
    }
    if same.broken {
        return Reason::NotBuiltByHydra {
            against,
            because: "meta.broken".to_owned(),
        };
    }
    if let Some(platforms) = &same.hydra_platforms
        && !platforms.iter().any(|platform| platform == &root.system)
    {
        return Reason::NotBuiltByHydra {
            against,
            because: if platforms.is_empty() {
                "meta.hydraPlatforms = [ ]".to_owned()
            } else {
                format!("meta.hydraPlatforms = [ {} ]", platforms.join(" "))
            },
        };
    }
    if facts.hello_served(&same.input) == Some(false) {
        return Reason::NewerThanCache { against };
    }
    unknown(&format!(
        "{against} builds exactly this, yet no cache has it; Hydra may have failed it"
    ))
}

/// The root differs from every candidate: diff it against the closest one
/// of its version, a cached one first.
fn overridden(store_dir: &Path, root: &Drv, candidates: &[Candidate], facts: &Facts) -> Reason {
    let version = root.version.as_deref();
    let same_version: Vec<&Candidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate.version.as_deref() == version || drv_name(&candidate.drv) == root.name
        })
        .collect();
    let best = same_version
        .iter()
        .filter_map(|candidate| {
            let theirs = StorePath::from_absolute_path(candidate.drv.as_bytes()).ok()?;
            let diff = Diff::read(store_dir, &theirs, &root.path).ok()?;
            let cached = facts.served(&candidate.drv).unwrap_or(false);
            Some((cached, diff, *candidate))
        })
        .min_by_key(|(cached, diff, _)| (!cached, diff.size()));
    if let Some((against_cached, diff, candidate)) = best {
        return Reason::Overridden {
            against: candidate.label(),
            against_cached,
            diff,
        };
    }
    let others: Vec<String> = candidates
        .iter()
        .map(|candidate| {
            let found = candidate.version.as_deref().unwrap_or("unversioned");
            format!("{} is {found}", candidate.label())
        })
        .collect();
    unknown(&format!(
        "no counterpart has version {}: {}",
        version.unwrap_or("none"),
        others.join(", ")
    ))
}

/// The derivation name in a `.drv` path: `hello-2.12` for
/// `/nix/store/<hash>-hello-2.12.drv`.
fn drv_name(drv: &str) -> &str {
    let base = drv.rsplit('/').next().unwrap_or(drv);
    let name = base.split_once('-').map_or(base, |(_, name)| name);
    name.strip_suffix(".drv").unwrap_or(name)
}

/// `urls` (space-separated) or `url`, in env or structured attrs.
fn fetch_urls(env: &std::collections::BTreeMap<String, String>) -> Vec<String> {
    let structured = env
        .get("__json")
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok());
    let from_json = |key: &str| -> Vec<String> {
        match structured.as_ref().and_then(|json| json.get(key)) {
            Some(serde_json::Value::Array(urls)) => urls
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect(),
            Some(serde_json::Value::String(url)) => vec![url.clone()],
            _ => Vec::new(),
        }
    };
    let from_env = |key: &str| -> Vec<String> {
        env.get(key)
            .map(|urls| urls.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default()
    };
    ["urls", "url"]
        .iter()
        .map(|key| {
            let found = from_env(key);
            if found.is_empty() {
                from_json(key)
            } else {
                found
            }
        })
        .find(|urls| !urls.is_empty())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "reason_tests.rs"]
mod tests;
