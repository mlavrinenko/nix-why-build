//! Explain what a Nix build would compile because no binary cache serves it.
//!
//! [`explain`] evaluates an installable, walks its derivation graph and asks
//! the substituters about each output, independent of the local store.
//!
//! ```no_run
//! # async fn run() -> Result<(), nix_why_build::Error> {
//! let options = nix_why_build::Options::new(".#nixosConfigurations.tank");
//! let report = nix_why_build::explain(&options).await?;
//! for root in &report.roots {
//!     println!("{}: {} {}", root.name, root.reason.category(), root.reason.detail());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Only the items re-exported below are public; the modules behind them are
//! not:
//!
//! ```compile_fail,E0603
//! use nix_why_build::walk;
//! ```

mod cache;
mod config;
mod counterparts;
mod diff;
mod drv;
mod eval;
mod human;
mod lookup;
mod modules;
mod reason;
mod report;
mod requester;
mod roots;
mod walk;

#[cfg(test)]
extern crate self as nix_why_build;
#[cfg(test)]
#[path = "../tests/support/fixture.rs"]
mod fixture_tests;

use std::path::PathBuf;

pub use cache::CacheError;
pub use diff::Diff;
pub use eval::EvalError;
pub use reason::Reason;
pub use report::{Cascade, Package, Report, Requested, Root, Unknown};

/// Compiles the README's library example, so it cannot rot.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// What to explain, and against which caches.
///
/// Build it with [`Options::new`] and the `with_*` methods; the struct is
/// `#[non_exhaustive]`, so a field added later breaks no caller.
///
/// ```
/// let options = nix_why_build::Options::new(".#nixosConfigurations.tank")
///     .with_substituters(vec!["https://cache.nixos.org".to_owned()])
///     .with_trusted_keys(vec!["cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=".to_owned()])
///     .with_store_dir("/nix/store");
/// assert_eq!(options.installable, ".#nixosConfigurations.tank");
/// assert_eq!(
///     options.substituters,
///     Some(vec!["https://cache.nixos.org".to_owned()])
/// );
/// assert_eq!(options.store_dir, std::path::PathBuf::from("/nix/store"));
/// ```
///
/// A struct literal does not compile outside this crate:
///
/// ```compile_fail,E0639
/// let _ = nix_why_build::Options {
///     installable: String::new(),
///     substituters: None,
///     trusted_keys: None,
///     store_dir: std::path::PathBuf::new(),
/// };
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Options {
    /// A flake installable (`.#nixosConfigurations.tank`) or a `.drv` path.
    pub installable: String,
    /// Substituter URLs; `None` asks `nix config show`.
    pub substituters: Option<Vec<String>>,
    /// Trusted public keys; `None` asks `nix config show`.
    pub trusted_keys: Option<Vec<String>>,
    /// Where `.drv` files live.
    pub store_dir: PathBuf,
}

impl Options {
    /// Explains `installable` against nix's own configuration.
    #[must_use]
    pub fn new(installable: &str) -> Self {
        Self {
            installable: installable.to_owned(),
            substituters: None,
            trusted_keys: None,
            store_dir: PathBuf::from("/nix/store"),
        }
    }

    /// Asks these substituters instead of nix's own list.
    #[must_use]
    pub fn with_substituters(mut self, urls: Vec<String>) -> Self {
        self.substituters = Some(urls);
        self
    }

    /// Trusts these `name:key` public keys instead of nix's own list.
    #[must_use]
    pub fn with_trusted_keys(mut self, keys: Vec<String>) -> Self {
        self.trusted_keys = Some(keys);
        self
    }

    /// Reads `.drv` files from `dir` instead of `/nix/store`.
    #[must_use]
    pub fn with_store_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.store_dir = dir.into();
        self
    }
}

/// Why [`explain`] failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Running or evaluating with nix failed.
    #[error(transparent)]
    Eval(#[from] eval::EvalError),
    /// `nix config show --json` printed something unexpected.
    #[error("nix config show: {0}")]
    Config(#[from] serde_json::Error),
    /// The substituter set could not be built.
    #[error(transparent)]
    Cache(#[from] cache::CacheError),
}

/// Evaluates the installable, walks it, and reports what gets built.
///
/// # Errors
///
/// [`Error::Eval`] when nix fails, [`Error::Config`] when its configuration
/// does not parse, [`Error::Cache`] for a malformed trusted key.
pub async fn explain(options: &Options) -> Result<Report, Error> {
    let nix_config = if options.substituters.is_some() && options.trusted_keys.is_some() {
        config::NixConfig {
            require_sigs: true,
            ..config::NixConfig::default()
        }
    } else {
        config::NixConfig::parse(&eval::nix(&["config", "show", "--json"]).await?)?
    };
    let urls = options
        .substituters
        .clone()
        .unwrap_or(nix_config.substituters);
    let keys = options
        .trusted_keys
        .clone()
        .unwrap_or(nix_config.trusted_keys);
    let substituters = cache::Substituters::new(&urls, &keys, nix_config.require_sigs)?;
    let config_attr = eval::config_attr(&options.installable);
    let walked = async {
        let top = eval::drv_path(&options.installable).await?;
        Ok::<_, Error>(walk::walk(&options.store_dir, &top, &substituters).await)
    };
    let indexed = async {
        match &config_attr {
            Some(attr) => Some(modules::load(attr).await),
            None => None,
        }
    };
    let labelled = async {
        match eval::flake_ref(&options.installable) {
            Some(flake) => Some(modules::labels(flake).await),
            None => None,
        }
    };
    let (graph, index, labels) = tokio::join!(walked, indexed, labelled);
    let graph = graph?;
    let split = roots::split(&graph);
    let mut report = Report::new(&graph, &split, &substituters);
    let flake = match &labels {
        Some(Ok(labels)) => lookup::Inputs::Listed(labels),
        Some(Err(reason)) => {
            report
                .warnings
                .push(format!("flake inputs unknown: {reason}"));
            lookup::Inputs::Unlisted(reason)
        }
        None => lookup::Inputs::NoFlake,
    };
    let (reasons, warnings) =
        lookup::classify_roots(&options.store_dir, &graph, &split, flake, &substituters).await;
    // Only now: the walk and the root lookup both ask the substituters.
    report.warnings.extend(substituters.warnings());
    report.warnings.extend(warnings);
    report.set_reasons(&reasons);
    match index {
        Some(Ok(index)) => report.locate(&index, &labels.and_then(Result::ok).unwrap_or_default()),
        Some(Err(reason)) => report
            .warnings
            .push(format!("module files unknown: {reason}")),
        None => {}
    }
    Ok(report)
}
