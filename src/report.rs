//! What [`crate::explain`] returns, and how the CLI prints it.

use nix_compat::store_path::StorePath;
use serde::Serialize;

use crate::cache::Substituters;
use crate::modules::{Index, Labels};
use crate::requester::{Parents, Request, requests};
use crate::roots::{Chain, Split};
use crate::walk::Graph;

/// How many cascades a root lists before it says how many more there are.
const SHOWN_CASCADES: usize = 6;

/// How many options a `wanted by` line names; lines run long with files.
const SHOWN_OPTIONS: usize = 2;

/// How many module files a `wanted by` entry names.
const SHOWN_FILES: usize = 2;

/// How many roots a group names before it says how many more there are.
const SHOWN_GROUP_ROOTS: usize = 3;

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

    /// Shown roots, grouped when they drag in the same named packages: a
    /// custom package's hundred vendored crates read as one line. A group
    /// names its first three roots; up to six named cascades follow.
    #[must_use]
    pub fn human(&self) -> String {
        let mut out = String::new();
        for group in self.groups() {
            out.push_str(&group.header());
            out.push('\n');
            let options = group.options();
            let options: Vec<&str> = options.iter().map(String::as_str).collect();
            if let Some(line) = listed("wanted by", &options, SHOWN_OPTIONS) {
                out.push_str(&line);
            }
            let names: Vec<&str> = group
                .cascades
                .iter()
                .map(|cascade| cascade.name.as_str())
                .collect();
            if let Some(line) = listed("drags in", &names, SHOWN_CASCADES) {
                out.push_str(&line);
            }
        }
        out
    }

    /// Shown roots, grouped by the named packages they drag in. Roots that
    /// drag in nothing named stand alone. Most named cascades first, then
    /// the largest group, then by name.
    #[must_use]
    pub fn groups(&self) -> Vec<Group<'_>> {
        let mut groups: Vec<Group<'_>> = Vec::new();
        for root in self.shown_roots() {
            let cascades: Vec<&Cascade> =
                root.cascades.iter().filter(|c| c.pname.is_some()).collect();
            let same = |group: &&mut Group<'_>| {
                !cascades.is_empty() && same_packages(&group.cascades, &cascades)
            };
            match groups.iter_mut().find(same) {
                Some(group) => group.roots.push(root),
                None => groups.push(Group {
                    roots: vec![root],
                    cascades,
                }),
            }
        }
        for group in &mut groups {
            group
                .roots
                .sort_by(|left, right| left.name.cmp(&right.name));
        }
        groups.sort_by(|left, right| {
            right
                .cascades
                .len()
                .cmp(&left.cascades.len())
                .then_with(|| right.roots.len().cmp(&left.roots.len()))
                .then_with(|| left.first_name().cmp(right.first_name()))
        });
        groups
    }

    /// Any warnings, one per line, then the one-line tally with the first
    /// unknown's reason if any.
    #[must_use]
    pub fn summary(&self) -> String {
        let warnings: String = self
            .warnings
            .iter()
            .map(|warning| format!("warning: {warning}\n"))
            .collect();
        let roots = self.shown_roots().count();
        let packages = self.to_build.iter().filter(|p| p.pname.is_some()).count();
        let rest = self.to_build.len() - packages;
        let groups = self.groups().len();
        let mut line = format!("{roots} {}", plural(roots, "root"));
        if groups < roots {
            line.push_str(&format!(" in {groups} {}", plural(groups, "group")));
        }
        line.push_str(&format!(
            ", {packages} {} to build, {rest} more without a pname",
            plural(packages, "package"),
        ));
        if let Some(first) = self.unknown.first() {
            line.push_str(&format!(
                ", {} unknown (first: {}: {})",
                self.unknown.len(),
                first.drv,
                first.reason
            ));
        }
        line.push('\n');
        warnings + &line
    }

    fn shown_roots(&self) -> impl Iterator<Item = &Root> {
        self.roots
            .iter()
            .filter(|root| root.pname.is_some() || named(&root.cascades) > 0)
    }
}

/// Roots that drag in the same named packages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group<'a> {
    /// The roots, by name.
    pub roots: Vec<&'a Root>,
    /// The named packages they drag in, nearest first.
    pub cascades: Vec<&'a Cascade>,
}

impl Group<'_> {
    /// The options pulling in any of the roots, once each; none when the
    /// toplevel is all there is to say.
    fn options(&self) -> Vec<String> {
        let mut options: Vec<String> = self
            .roots
            .iter()
            .flat_map(|root| &root.requested_by)
            .map(wanted_by)
            .collect();
        options.sort_unstable_by(|left, right| {
            rank(left).cmp(&rank(right)).then_with(|| left.cmp(right))
        });
        options.dedup();
        if options == ["toplevel"] {
            options.clear();
        }
        options
    }

    fn first_name(&self) -> &str {
        self.roots.first().map_or("", |root| root.name.as_str())
    }

    /// `name` (with ` (fetch)`) for one root; `N roots: a, b, c (+M more)`
    /// for several, `fetches` when every one is.
    fn header(&self) -> String {
        let [root] = self.roots.as_slice() else {
            let kind = if self.roots.iter().all(|root| root.fixed_output) {
                "fetches"
            } else {
                "roots"
            };
            let names: Vec<&str> = self.roots.iter().map(|root| root.name.as_str()).collect();
            let shown = names.get(..SHOWN_GROUP_ROOTS).unwrap_or(&names);
            let mut line = format!("{} {kind}: {}", names.len(), shown.join(", "));
            if names.len() > shown.len() {
                line.push_str(&format!(" (+{} more)", names.len() - shown.len()));
            }
            return line;
        };
        if root.fixed_output {
            format!("{} (fetch)", root.name)
        } else {
            root.name.clone()
        }
    }
}

/// Package lists first, since they name the package itself; then units; then
/// files and anything else, which often only mention it.
fn rank(option: &str) -> u8 {
    let option = option.split(" (").next().unwrap_or(option);
    if option.ends_with("environment.systemPackages") || option.ends_with("home.packages") {
        0
    } else if option.contains("systemd.") {
        1
    } else {
        2
    }
}

/// `option (file, file +N)`: the option, with up to two files that set it.
fn wanted_by(request: &Requested) -> String {
    let Some(shown) = request.files.get(..SHOWN_FILES.min(request.files.len())) else {
        return request.option.clone();
    };
    if shown.is_empty() {
        return request.option.clone();
    }
    let mut files = shown.join(", ");
    if request.files.len() > shown.len() {
        files.push_str(&format!(" +{}", request.files.len() - shown.len()));
    }
    format!("{} ({files})", request.option)
}

fn same_packages(left: &[&Cascade], right: &[&Cascade]) -> bool {
    sorted_drvs(left) == sorted_drvs(right)
}

fn sorted_drvs<'a>(cascades: &[&'a Cascade]) -> Vec<&'a str> {
    let mut drvs: Vec<&str> = cascades
        .iter()
        .map(|cascade| cascade.drv.as_str())
        .collect();
    drvs.sort_unstable();
    drvs
}

fn named(cascades: &[Cascade]) -> usize {
    cascades
        .iter()
        .filter(|cascade| cascade.pname.is_some())
        .count()
}

fn listed(label: &str, names: &[&str], cap: usize) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let shown = names.get(..cap).unwrap_or(names);
    let mut line = format!("  {label} {}", shown.join(", "));
    if names.len() > shown.len() {
        line.push_str(&format!(" (+{} more)", names.len() - shown.len()));
    }
    line.push('\n');
    Some(line)
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

fn plural(count: usize, word: &str) -> String {
    if count == 1 {
        word.to_owned()
    } else {
        format!("{word}s")
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
