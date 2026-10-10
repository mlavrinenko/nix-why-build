//! What [`crate::explain`] returns, and how the CLI prints it.

use serde::Serialize;

use crate::cache::Substituters;
use crate::walk::Graph;

/// The answer: what gets built, and what could not be decided.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The `.drv` the walk started from.
    pub toplevel: String,
    /// The substituters asked, in order.
    pub substituters: Vec<String>,
    /// Derivations no substituter serves, sorted by name.
    pub to_build: Vec<Package>,
    /// Derivations with no answer.
    pub unknown: Vec<Unknown>,
}

/// One derivation that gets built.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Package {
    /// The derivation name, `pname-version` for a package.
    pub name: String,
    /// `pname`; config glue has none.
    pub pname: Option<String>,
    /// `version`.
    pub version: Option<String>,
    /// The `.drv` path.
    pub drv: String,
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
    /// Collects the report from a finished walk.
    #[must_use]
    pub fn new(graph: &Graph, substituters: &Substituters) -> Self {
        Self {
            toplevel: graph.top.to_absolute_path(),
            substituters: substituters.urls().into_iter().map(str::to_owned).collect(),
            to_build: graph
                .to_build()
                .into_iter()
                .map(|drv| Package {
                    name: drv.name.clone(),
                    pname: drv.pname.clone(),
                    version: drv.version.clone(),
                    drv: drv.path.to_absolute_path(),
                })
                .collect(),
            unknown: graph
                .unknown()
                .into_iter()
                .map(|(path, reason)| Unknown {
                    drv: path.to_absolute_path(),
                    reason: reason.to_owned(),
                })
                .collect(),
        }
    }

    /// One package per line: everything to build that has a `pname`.
    #[must_use]
    pub fn human(&self) -> String {
        self.to_build
            .iter()
            .filter(|package| package.pname.is_some())
            .map(|package| format!("{}\n", package.name))
            .collect()
    }

    /// The one-line tally, with the first unknown's reason if any.
    #[must_use]
    pub fn summary(&self) -> String {
        let packages = self.to_build.iter().filter(|p| p.pname.is_some()).count();
        let glue = self.to_build.len() - packages;
        let mut line = format!(
            "{packages} to build, plus {glue} config {}",
            plural(glue, "derivation")
        );
        if let Some(first) = self.unknown.first() {
            line.push_str(&format!(
                ", {} unknown (first: {}: {})",
                self.unknown.len(),
                first.drv,
                first.reason
            ));
        }
        line.push('\n');
        line
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
mod tests {
    use super::{Package, Report, Unknown};

    fn package(name: &str, pname: Option<&str>) -> Package {
        Package {
            name: name.to_owned(),
            pname: pname.map(str::to_owned),
            version: None,
            drv: format!("/nix/store/x-{name}.drv"),
        }
    }

    fn report(to_build: Vec<Package>, unknown: Vec<Unknown>) -> Report {
        Report {
            toplevel: "/nix/store/x-system.drv".to_owned(),
            substituters: vec![],
            to_build,
            unknown,
        }
    }

    #[test]
    fn human_lists_packages_and_skips_glue() {
        let report = report(
            vec![
                package("app-1.0", Some("app")),
                package("etc", None),
                package("zlib-1.3", Some("zlib")),
            ],
            vec![],
        );
        assert_eq!(report.human(), "app-1.0\nzlib-1.3\n");
        assert_eq!(report.summary(), "2 to build, plus 1 config derivation\n");
    }

    #[test]
    fn summary_counts_glue_in_plural_and_names_the_first_unknown() {
        let report = report(
            vec![package("etc", None), package("system-path", None)],
            vec![
                Unknown {
                    drv: "/nix/store/a-x.drv".to_owned(),
                    reason: "down".to_owned(),
                },
                Unknown {
                    drv: "/nix/store/b-y.drv".to_owned(),
                    reason: "gone".to_owned(),
                },
            ],
        );
        assert_eq!(report.human(), "");
        assert_eq!(
            report.summary(),
            "0 to build, plus 2 config derivations, 2 unknown (first: /nix/store/a-x.drv: down)\n"
        );
    }
}
