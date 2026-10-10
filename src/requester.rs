//! Who asks for a root: the glue derivations above it that an option builds.

use std::collections::{HashMap, HashSet, VecDeque};

use nix_compat::store_path::StorePath;

use crate::walk::Graph;

/// One way a root gets requested.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// The option the glue node stands for, like `environment.systemPackages`,
    /// or `toplevel` when the way up meets no glue node.
    pub option: String,
    /// The glue derivation (or the toplevel).
    pub glue: StorePath,
    /// The derivation right below the glue node: what the option lists.
    pub item: StorePath,
}

/// Reverse build edges: who gets built because of whom.
#[derive(Clone, Debug, Default)]
pub struct Parents(HashMap<StorePath, Vec<StorePath>>);

impl Parents {
    /// Inverts the build edges of `graph`.
    #[must_use]
    pub fn of(graph: &Graph) -> Self {
        let mut parents: HashMap<StorePath, Vec<StorePath>> = HashMap::new();
        for parent in graph.builds() {
            for child in graph.build_inputs(parent) {
                parents
                    .entry(child.clone())
                    .or_default()
                    .push(parent.clone());
            }
        }
        for list in parents.values_mut() {
            list.sort_by_key(ToString::to_string);
        }
        Self(parents)
    }

    fn get(&self, path: &StorePath) -> &[StorePath] {
        self.0.get(path).map_or(&[], Vec::as_slice)
    }
}

/// The requests for `root`, sorted by option: walking up from it, the first
/// glue node on every way to the toplevel.
#[must_use]
pub fn requests(graph: &Graph, parents: &Parents, root: &StorePath) -> Vec<Request> {
    let mut seen: HashSet<&StorePath> = HashSet::from([root]);
    let mut queue = VecDeque::from([root]);
    let mut found = Vec::new();
    while let Some(node) = queue.pop_front() {
        for parent in parents.get(node) {
            let option = glue_option(graph, parents, parent)
                .or_else(|| (parent == &graph.top).then(|| "toplevel".to_owned()));
            if let Some(option) = option {
                found.push(Request {
                    option,
                    glue: parent.clone(),
                    item: node.clone(),
                });
            } else if seen.insert(parent) {
                queue.push_back(parent);
            }
        }
    }
    found.sort_by(|left, right| {
        left.option
            .cmp(&right.option)
            .then_with(|| graph.name(&left.item).cmp(graph.name(&right.item)))
    });
    found.dedup();
    found
}

/// The option a glue derivation stands for, by its name.
fn glue_option(graph: &Graph, parents: &Parents, glue: &StorePath) -> Option<String> {
    let name = graph.name(glue);
    let fixed = match name {
        "system-path" => Some("environment.systemPackages"),
        "etc" => Some("environment.etc"),
        "home-manager-path" => Some("home.packages"),
        "home-manager-files" => Some("home.file"),
        "user-environment" => Some("users.users.*.packages"),
        _ => None,
    };
    if let Some(option) = fixed {
        return Some(option.to_owned());
    }
    let user = parents
        .get(glue)
        .iter()
        .any(|parent| graph.name(parent) == "user-units");
    unit_option(name, user)
}

/// `systemd[.user].<kind>s.<unit>` for a `unit-<unit>.<kind>` derivation.
fn unit_option(name: &str, user: bool) -> Option<String> {
    let (unit, kind) = unit_parts(name)?;
    let scope = if user { "systemd.user" } else { "systemd" };
    Some(format!("{scope}.{kind}.{}", attr_name(unit)))
}

/// The unit name and the plural option kind (`services`) of a
/// `unit-<unit>.<kind>` derivation name.
pub(crate) fn unit_parts(name: &str) -> Option<(&str, &'static str)> {
    let (unit, kind) = name.strip_prefix("unit-")?.rsplit_once('.')?;
    let kind = match kind {
        "service" => "services",
        "socket" => "sockets",
        "timer" => "timers",
        "path" => "paths",
        "mount" => "mounts",
        "automount" => "automounts",
        "slice" => "slices",
        "target" => "targets",
        _ => return None,
    };
    Some((unit, kind))
}

/// `name` as a Nix attribute: bare when it is an identifier, else quoted.
pub(crate) fn attr_name(name: &str) -> String {
    let bare = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '\''));
    if bare {
        name.to_owned()
    } else {
        format!("{name:?}")
    }
}

#[cfg(test)]
#[path = "requester_tests.rs"]
mod tests;
