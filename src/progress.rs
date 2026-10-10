//! What [`crate::explain`] reports while it works. The library prints
//! nothing itself; the caller passes a callback and draws the phases its own
//! way.

use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;

/// A phase of [`crate::explain`], reported to the callback set with
/// [`Options::with_progress`](crate::Options::with_progress). `Display` is a
/// short line for it, like `asking 5 caches: level 7, 3210 derivations`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Progress {
    /// nix is evaluating the installable down to its `.drv`; on a big
    /// configuration this is the longest wait.
    Evaluating {
        /// The installable, as given.
        installable: String,
    },
    /// The walk is asking the caches about one level of derivations.
    Walking {
        /// Which level, counting from 1 at the toplevel.
        level: usize,
        /// Derivations read so far, this level included.
        derivations: usize,
        /// How many substituters are asked.
        caches: usize,
    },
    /// The roots are being compared with their nixpkgs counterparts.
    Comparing {
        /// How many roots.
        roots: usize,
    },
}

impl fmt::Display for Progress {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluating { installable } => write!(out, "evaluating {installable}"),
            Self::Walking {
                level,
                derivations,
                caches,
            } => write!(
                out,
                "asking {}: level {level}, {}",
                counted(*caches, "cache"),
                counted(*derivations, "derivation"),
            ),
            Self::Comparing { roots } => {
                write!(out, "comparing {} with nixpkgs", counted(*roots, "root"))
            }
        }
    }
}

/// `1 cache`, `5 caches`.
fn counted(count: usize, noun: &str) -> String {
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{plural}")
}

/// The caller's callback. It is shared between the tasks `explain` runs, and
/// `Options` stays `Clone` and `Debug` with it inside.
#[derive(Clone)]
pub(crate) struct Callback(Arc<dyn Fn(Progress) + Send + Sync>);

impl Callback {
    pub(crate) fn new(report: impl Fn(Progress) + Send + Sync + 'static) -> Self {
        Self(Arc::new(report))
    }

    pub(crate) fn call(&self, progress: Progress) {
        (self.0)(progress);
    }
}

impl fmt::Debug for Callback {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("Callback(..)")
    }
}

/// Runs `work`, logging at `info` how long `phase` took.
pub(crate) async fn timed<T>(phase: &str, work: impl Future<Output = T>) -> T {
    let started = Instant::now();
    let done = work.await;
    log::info!("{phase} took {:?}", started.elapsed());
    done
}

#[cfg(test)]
mod tests {
    use super::Progress;

    #[test]
    fn each_phase_reads_as_one_line() {
        let lines: Vec<String> = [
            Progress::Evaluating {
                installable: ".#nixosConfigurations.tank".to_owned(),
            },
            Progress::Walking {
                level: 7,
                derivations: 3210,
                caches: 5,
            },
            Progress::Comparing { roots: 52 },
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            lines,
            [
                "evaluating .#nixosConfigurations.tank",
                "asking 5 caches: level 7, 3210 derivations",
                "comparing 52 roots with nixpkgs",
            ]
        );
    }

    #[test]
    fn one_of_anything_is_singular() {
        let walking = Progress::Walking {
            level: 1,
            derivations: 1,
            caches: 1,
        };
        assert_eq!(walking.to_string(), "asking 1 cache: level 1, 1 derivation");
        assert_eq!(
            Progress::Comparing { roots: 1 }.to_string(),
            "comparing 1 root with nixpkgs"
        );
    }
}
