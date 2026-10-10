//! How the CLI prints a [`Report`]: roots grouped by what they drag in, each
//! with why it misses and who wants it, then a one-line tally.

use crate::reason::Reason;
use crate::report::{Cascade, Report, Requested, Root, named};

/// How many cascades a root lists before it says how many more there are.
const SHOWN_CASCADES: usize = 6;

/// How many options a `wanted by` line names; lines run long with files.
const SHOWN_OPTIONS: usize = 2;

/// How many module files a `wanted by` entry names.
const SHOWN_FILES: usize = 2;

/// How many roots a group names before it says how many more there are.
const SHOWN_GROUP_ROOTS: usize = 3;

impl Report {
    /// Shown roots, grouped when they drag in the same named packages: a
    /// custom package's hundred vendored crates read as one line. Local-only
    /// roots that drag in nothing named share one group, since no cache can
    /// fix them. A group names its first three roots; up to six named
    /// cascades follow.
    #[must_use]
    pub fn human(&self) -> String {
        let mut out = String::new();
        for group in self.groups() {
            out.push_str(&group.header());
            out.push('\n');
            out.push_str(&format!("  why {}\n", group.why()));
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

    /// Shown roots, grouped by the named packages they drag in. Local-only
    /// roots that drag in nothing named form one group (see
    /// [`Group::header`]); other roots that drag in nothing named stand
    /// alone. The local-only group comes last, as nothing in it can become
    /// a cache hit; the rest go most named cascades first, then the largest
    /// group, then by name.
    #[must_use]
    pub fn groups(&self) -> Vec<Group<'_>> {
        let mut groups: Vec<Group<'_>> = Vec::new();
        for root in self.shown_roots() {
            let cascades: Vec<&Cascade> =
                root.cascades.iter().filter(|c| c.pname.is_some()).collect();
            let same = |group: &&mut Group<'_>| {
                if cascades.is_empty() {
                    local_only(root) && group.is_local_only()
                } else {
                    same_packages(&group.cascades, &cascades)
                }
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
            left.is_local_only()
                .cmp(&right.is_local_only())
                .then_with(|| right.cascades.len().cmp(&left.cascades.len()))
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
    /// One root's reason in full; for several, the reason when they share it,
    /// else each category with its count.
    fn why(&self) -> String {
        let lines: Vec<String> = self
            .roots
            .iter()
            .map(|root| format!("{}: {}", root.reason.category(), root.reason.detail()))
            .collect();
        if let Some(first) = lines.first()
            && lines.iter().all(|line| line == first)
        {
            return first.clone();
        }
        let mut counts: Vec<(&str, usize)> = Vec::new();
        for root in &self.roots {
            let category = root.reason.category();
            match counts.iter_mut().find(|(seen, _)| *seen == category) {
                Some((_, count)) => *count += 1,
                None => counts.push((category, 1)),
            }
        }
        counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
        counts
            .iter()
            .map(|(category, count)| format!("{category} ({count})"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The options pulling in any of the roots, once each with the files of
    /// every root that sets it, first seen first; none when the toplevel is
    /// all there is to say.
    fn options(&self) -> Vec<String> {
        let mut merged: Vec<Requested> = Vec::new();
        for request in self.roots.iter().flat_map(|root| &root.requested_by) {
            let Some(seen) = merged.iter_mut().find(|seen| seen.option == request.option) else {
                merged.push(request.clone());
                continue;
            };
            for file in &request.files {
                if !seen.files.contains(file) {
                    seen.files.push(file.clone());
                }
            }
        }
        let mut options: Vec<String> = merged.iter().map(wanted_by).collect();
        options.sort_unstable_by(|left, right| {
            rank(left).cmp(&rank(right)).then_with(|| left.cmp(right))
        });
        if options == ["toplevel"] {
            options.clear();
        }
        options
    }

    /// Whether every root is local-only and drags in nothing named: the
    /// group nix builds by design, whatever a cache holds.
    fn is_local_only(&self) -> bool {
        self.cascades.is_empty() && self.roots.iter().all(|root| local_only(root))
    }

    fn first_name(&self) -> &str {
        self.roots.first().map_or("", |root| root.name.as_str())
    }

    /// `name` (with ` (fetch)`) for one root; `N roots: a, b, c (+M more)`
    /// for several, `fetches` when every one is, `local-only` for the
    /// local-only group.
    fn header(&self) -> String {
        let [root] = self.roots.as_slice() else {
            let kind = if self.is_local_only() {
                "local-only"
            } else if self.roots.iter().all(|root| root.fixed_output) {
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

fn local_only(root: &Root) -> bool {
    matches!(root.reason, Reason::LocalOnly)
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

fn plural(count: usize, word: &str) -> String {
    if count == 1 {
        word.to_owned()
    } else {
        format!("{word}s")
    }
}
