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
mod progress;
mod reason;
mod report;
mod requester;
mod roots;
mod style;
mod walk;

#[cfg(test)]
extern crate self as nix_why_build;
#[cfg(test)]
#[path = "../tests/support/fixture.rs"]
mod fixture_tests;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use progress::{Callback, timed};

pub use cache::CacheError;
pub use diff::Diff;
pub use eval::EvalError;
pub use progress::Progress;
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
    progress: Option<Callback>,
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
            progress: None,
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

    /// Tells `report` each phase as it starts: nix evaluating, the walk
    /// asking the caches level by level, the roots being compared. The
    /// library prints nothing itself, so a caller draws the phases its own
    /// way. The callback runs on whichever thread is working, so keep it
    /// short.
    ///
    /// ```
    /// let options = nix_why_build::Options::new(".#nixosConfigurations.tank")
    ///     .with_progress(|progress| eprintln!("{progress}"));
    /// ```
    #[must_use]
    pub fn with_progress(mut self, report: impl Fn(Progress) + Send + Sync + 'static) -> Self {
        self.progress = Some(Callback::new(report));
        self
    }

    /// Tells the callback, if any, that `progress` has started.
    pub(crate) fn notify(&self, progress: Progress) {
        if let Some(callback) = &self.progress {
            callback.call(progress);
        }
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
    let substituters = substituters(options).await?;
    let gathered = gather(options, &substituters).await;
    let graph = gathered.graph?;
    let split = roots::split(&graph);
    let mut report = Report::new(&graph, &split, &substituters);
    let flake = match &gathered.labels {
        Some(Ok(labels)) => lookup::Inputs::Listed(labels),
        Some(Err(reason)) => {
            report
                .warnings
                .push(format!("flake inputs unknown: {reason}"));
            lookup::Inputs::Unlisted(reason)
        }
        None => lookup::Inputs::NoFlake,
    };
    let (reasons, warnings) = timed(
        "comparing roots with nixpkgs",
        lookup::classify_roots(options, &graph, &split, flake, &substituters),
    )
    .await;
    // Only now: the walk and the root lookup both ask the substituters.
    report.warnings.extend(substituters.warnings());
    report.warnings.extend(warnings);
    report.set_reasons(&reasons);
    match gathered.index {
        Some(Ok(index)) => report.locate(
            &index,
            &gathered.labels.and_then(Result::ok).unwrap_or_default(),
        ),
        Some(Err(reason)) => report
            .warnings
            .push(format!("module files unknown: {reason}")),
        None => {}
    }
    Ok(report)
}

/// The substituters to ask: the options', else nix's own configuration.
async fn substituters(options: &Options) -> Result<cache::Substituters, Error> {
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
    Ok(cache::Substituters::new(
        &urls,
        &keys,
        nix_config.require_sigs,
    )?)
}

/// What the three side-by-side nix jobs found. `index` and `labels` are
/// `None` when the installable has no configuration or flake to ask about.
struct Gathered {
    graph: Result<walk::Graph, Error>,
    index: Option<Result<modules::Index, String>>,
    labels: Option<Result<modules::Labels, String>>,
}

/// Walks the installable while nix indexes its options and lists its flake's
/// inputs. Only the walk reports progress: it is the one in the way.
async fn gather(options: &Options, substituters: &cache::Substituters) -> Gathered {
    let config_attr = eval::config_attr(&options.installable);
    let index_pending = AtomicBool::new(config_attr.is_some());
    let walked = async {
        options.notify(Progress::Evaluating {
            installable: options.installable.clone(),
        });
        let top = timed("evaluating", eval::drv_path(&options.installable)).await?;
        let caches = substituters.urls().len();
        let on_level = |level: walk::Level| {
            options.notify(Progress::Walking {
                level: level.number,
                derivations: level.derivations,
                caches,
            });
        };
        let walk = walk::walk(&options.store_dir, &top, substituters, &on_level);
        let graph = timed("walking", walk).await;
        // The option index runs beside the walk and may outlast it.
        if index_pending.load(Ordering::Relaxed) {
            options.notify(Progress::Indexing);
        }
        Ok::<_, Error>(graph)
    };
    let indexed = async {
        let attr = config_attr.as_ref()?;
        let loaded = timed("indexing options", modules::load(attr)).await;
        index_pending.store(false, Ordering::Relaxed);
        Some(loaded)
    };
    let labelled = async {
        match eval::flake_ref(&options.installable) {
            Some(flake) => Some(timed("listing flake inputs", modules::labels(flake)).await),
            None => None,
        }
    };
    let (graph, index, labels) = tokio::join!(walked, indexed, labelled);
    Gathered {
        graph,
        index,
        labels,
    }
}
