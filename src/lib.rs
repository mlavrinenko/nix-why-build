//! Explain what a Nix build would compile because no binary cache serves it.
//!
//! [`explain`] evaluates an installable, walks its derivation graph and asks
//! the substituters about each output, independent of the local store.
//!
//! ```no_run
//! # async fn run() -> Result<(), nix_why_build::Error> {
//! let options = nix_why_build::Options::new(".#nixosConfigurations.tank");
//! let report = nix_why_build::explain(&options).await?;
//! for package in report.to_build.iter().filter(|p| p.pname.is_some()) {
//!     println!("{}", package.name);
//! }
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod config;
pub mod counterparts;
pub mod diff;
pub mod drv;
pub mod eval;
pub mod modules;
pub mod report;
pub mod requester;
pub mod roots;
pub mod walk;

#[cfg(test)]
extern crate self as nix_why_build;
#[cfg(test)]
#[path = "../tests/support/fixture.rs"]
mod fixture_tests;

use std::path::PathBuf;

pub use report::Report;

/// What to explain, and against which caches.
#[derive(Clone, Debug)]
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
}

/// Why [`explain`] failed.
#[derive(Debug, thiserror::Error)]
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
            Some(flake) if config_attr.is_some() => modules::labels(flake).await.ok(),
            _ => None,
        }
    };
    let (graph, index, labels) = tokio::join!(walked, indexed, labelled);
    let graph = graph?;
    let mut report = Report::new(&graph, &roots::split(&graph), &substituters);
    match index {
        Some(Ok(index)) => report.locate(&index, &labels.unwrap_or_default()),
        Some(Err(reason)) => report
            .warnings
            .push(format!("module files unknown: {reason}")),
        None => {}
    }
    Ok(report)
}
