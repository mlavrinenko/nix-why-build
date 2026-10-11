//! Which module files set the option that pulls a root in.
//!
//! One `nix eval` of the configuration, run beside the toplevel's, indexes
//! every option that can pull a package in ([`INDEX_EXPR`]); matching a
//! request against it is plain Rust.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::requester::{attr_name, unit_parts};

/// The Nix function `nix eval --apply` runs on the configuration.
pub const INDEX_EXPR: &str = include_str!("index.nix");

/// Every indexed option, per scope: the configuration, then each
/// home-manager user.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Index {
    /// The configuration first, then one scope per home-manager user.
    pub scopes: Vec<Scope>,
}

/// One module system's options.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Scope {
    /// Prepended to its option names: empty, or `home-manager.users.<u>.`.
    pub prefix: String,
    /// The glue `.drv` paths this scope builds (`home.path`, `home-files`).
    pub glue: Vec<String>,
    /// List options: each definition and the `.drv` paths it lists.
    pub lists: BTreeMap<String, Vec<Listing>>,
    /// Attribute-set options: each definition and the names it sets.
    pub attrs: BTreeMap<String, Vec<Naming>>,
    /// File options (`environment.etc`, `home.file`): entries and setters.
    pub sources: BTreeMap<String, Sources>,
}

/// One definition of a list option.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Listing {
    /// The module file.
    pub file: String,
    /// The `.drv` path of each package it lists.
    pub drvs: Vec<String>,
}

/// One definition of an attribute-set option.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Naming {
    /// The module file.
    pub file: String,
    /// The attribute names it sets.
    pub names: Vec<String>,
}

/// A file option: what each entry's `source` refers to, and who sets it.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Sources {
    /// Entry name to the `.drv` paths its `source` refers to.
    pub entries: BTreeMap<String, Vec<String>>,
    /// Each definition and the entries it sets.
    pub definitions: Vec<Naming>,
}

/// A request resolved against the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    /// The full option, like `systemd.services.nix-daemon` or
    /// `home-manager.users.me.xdg.configFile."fonts/fonts.conf"`.
    pub option: String,
    /// The module files that set it, as the index has them.
    pub files: Vec<String>,
}

impl Index {
    /// Resolves one request: `option` as the requester named it, the glue
    /// node's name and `.drv`, and the item's `.drv`. `None` for an option the
    /// index does not cover, like `toplevel`.
    #[must_use]
    pub fn locate(
        &self,
        option: &str,
        glue_name: &str,
        glue_drv: &str,
        item_drv: &str,
    ) -> Option<Located> {
        let scope = self
            .scopes
            .iter()
            .find(|scope| scope.glue.iter().any(|drv| drv == glue_drv))
            .or_else(|| self.scopes.iter().find(|scope| scope.prefix.is_empty()))?;
        match option {
            "environment.systemPackages" | "home.packages" => Some(scope.listing(option, item_drv)),
            "environment.etc" => scope.source(&["environment.etc"], item_drv),
            "home.file" => scope.source(&["xdg.configFile", "xdg.dataFile", "home.file"], item_drv),
            _ => scope.unit(option, glue_name),
        }
    }
}

impl Scope {
    fn listing(&self, option: &str, item_drv: &str) -> Located {
        let files = self
            .lists
            .get(option)
            .map(|definitions| {
                definitions
                    .iter()
                    .filter(|definition| definition.drvs.iter().any(|drv| drv == item_drv))
                    .map(|definition| definition.file.clone())
                    .collect()
            })
            .unwrap_or_default();
        Located {
            option: format!("{}{option}", self.prefix),
            files,
        }
    }

    fn unit(&self, option: &str, glue_name: &str) -> Option<Located> {
        let (unit, kind) = unit_parts(glue_name)?;
        let scope = if option.starts_with("systemd.user.") {
            "systemd.user"
        } else {
            "systemd"
        };
        let base = format!("{scope}.{kind}");
        let definitions = self.attrs.get(&base)?;
        let name = definitions
            .iter()
            .flat_map(|definition| &definition.names)
            .find(|name| name.as_str() == unit || path_safe(name) == unit)?;
        Some(Located {
            option: format!("{}{base}.{}", self.prefix, attr_name(name)),
            files: setters(definitions, name),
        })
    }

    fn source(&self, options: &[&str], item_drv: &str) -> Option<Located> {
        options.iter().find_map(|option| {
            let sources = self.sources.get(*option)?;
            let (name, _) = sources
                .entries
                .iter()
                .find(|(_, drvs)| drvs.iter().any(|drv| drv == item_drv))?;
            Some(Located {
                option: format!("{}{option}.{}", self.prefix, attr_name(name)),
                files: setters(&sources.definitions, name),
            })
        })
    }
}

fn setters(definitions: &[Naming], name: &str) -> Vec<String> {
    definitions
        .iter()
        .filter(|definition| definition.names.iter().any(|set| set == name))
        .map(|definition| definition.file.clone())
        .collect()
}

/// NixOS's `mkPathSafeName`: how a unit name becomes a store path name.
fn path_safe(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '[' | ']'))
        .map(|c| {
            if matches!(c, '@' | ':' | '\\') {
                '-'
            } else {
                c
            }
        })
        .collect()
}

/// Evaluates [`INDEX_EXPR`] on `config_attr`; the error is a sentence for a
/// report warning.
///
/// # Errors
///
/// Why nix failed, or why its output is not an index.
pub async fn load(config_attr: &str) -> Result<Index, String> {
    let json = crate::eval::nix(&["eval", "--json", config_attr, "--apply", INDEX_EXPR])
        .await
        .map_err(|err| crate::eval::first_error(&err.to_string()))?;
    serde_json::from_str(&json).map_err(|err| err.to_string())
}

/// Labels for the files of `flake` and its inputs.
///
/// # Errors
///
/// Why `nix flake archive` failed, or why its output does not parse.
pub async fn labels(flake: &str) -> Result<Labels, String> {
    let json = crate::eval::nix(&["flake", "archive", "--json", "--dry-run", flake])
        .await
        .map_err(|err| crate::eval::first_error(&err.to_string()))?;
    Labels::parse(&json).map_err(|err| err.to_string())
}

/// Short labels for module files: relative to the configuration's flake, or
/// `<input>:` and the path inside that flake input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Labels {
    /// Source store path to its label: empty for the flake itself.
    roots: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct Archive {
    path: String,
    #[serde(default)]
    inputs: BTreeMap<String, Archive>,
}

impl Labels {
    /// Reads `nix flake archive --json --dry-run` output.
    ///
    /// # Errors
    ///
    /// The JSON error when it is not that shape.
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        let archive: Archive = serde_json::from_str(json)?;
        let mut roots = Vec::new();
        collect(&archive, "", &mut roots);
        Ok(Self { roots })
    }

    /// The flake and its direct inputs that look like nixpkgs: their source
    /// holds `pkgs/top-level/all-packages.nix`. The flake itself is labelled
    /// `self`. Inputs of inputs are left out: each costs an import of nixpkgs,
    /// and what a configuration builds comes from its own inputs.
    #[must_use]
    pub fn nixpkgs_inputs(&self) -> Vec<crate::counterparts::Input> {
        self.roots
            .iter()
            .filter(|(path, label)| {
                !label.contains('/')
                    && Path::new(path)
                        .join("pkgs/top-level/all-packages.nix")
                        .is_file()
            })
            .map(|(path, label)| crate::counterparts::Input {
                label: if label.is_empty() {
                    "self".to_owned()
                } else {
                    label.clone()
                },
                path: path.clone(),
            })
            .collect()
    }

    /// `apps/x.nix` for the flake's own file, `nixpkgs:nixos/…` for an
    /// input's, the file as given for anything else.
    #[must_use]
    pub fn label(&self, file: &str) -> String {
        self.roots
            .iter()
            .find_map(|(path, label)| {
                let rel = file.strip_prefix(path.as_str())?.strip_prefix('/')?;
                Some(if label.is_empty() {
                    rel.to_owned()
                } else {
                    format!("{label}:{rel}")
                })
            })
            .unwrap_or_else(|| file.to_owned())
    }
}

/// Breadth first, so a store path shared through `follows` keeps its
/// shortest label.
fn collect(archive: &Archive, label: &str, roots: &mut Vec<(String, String)>) {
    let mut queue = std::collections::VecDeque::from([(archive, label.to_owned())]);
    while let Some((node, label)) = queue.pop_front() {
        if roots.iter().all(|(path, _)| path != &node.path) {
            roots.push((node.path.clone(), label.clone()));
        }
        for (name, input) in &node.inputs {
            let child = if label.is_empty() {
                name.clone()
            } else {
                format!("{label}/{name}")
            };
            queue.push_back((input, child));
        }
    }
}

#[cfg(test)]
#[path = "modules_tests.rs"]
mod tests;
