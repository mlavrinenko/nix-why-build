//! Ask binary caches whether they serve a store path.
//!
//! A path counts as served when a substituter returns a narinfo for it that a
//! trusted key signed (or any narinfo, with `require-sigs` off): the same test
//! nix applies before it substitutes instead of building.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use nix_derivation::{StoreDir, StorePath};
use nix_narinfo::{NarInfo, TrustedPublicKey};
use serde::Serialize;

/// Whether some substituter serves a store path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Availability {
    /// The first substituter, in configured order, with a trusted narinfo.
    Served {
        /// The substituter URL as configured.
        substituter: String,
    },
    /// Every substituter answered, and none serves it.
    Missing,
    /// Some substituter could not answer, and none serves it.
    Unknown {
        /// What went wrong, for the first substituter that failed.
        reason: String,
    },
}

/// Errors building a [`Substituters`] set.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// A trusted public key does not parse as `name:base64`.
    #[error("trusted key {key:?}: {reason}")]
    Key {
        /// The key as configured.
        key: String,
        /// Why it does not parse.
        reason: String,
    },
    /// The HTTP client could not start.
    #[error("http client: {0}")]
    Client(#[from] reqwest::Error),
}

/// The substituters to ask, in order, and the keys whose signatures count.
#[derive(Debug)]
pub struct Substituters {
    list: Vec<Entry>,
    unsupported: Vec<String>,
    keys: Vec<TrustedPublicKey>,
    require_sigs: bool,
    concurrency: usize,
    patience: std::time::Duration,
    client: reqwest::Client,
}

/// How long a connection (TCP and TLS handshake) may take to open.
///
/// Without a limit a host that drops packets keeps every probe waiting for
/// the whole patience, 1.5 s then 12 s. Five seconds is several round trips on
/// a bad link, so a cache that works is not given up on, and it stays under
/// the 12 s retry, so a blackholed host fails as unreachable, not as a stall.
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// One substituter to ask.
#[derive(Debug)]
struct Entry {
    url: String,
    location: Location,
    /// Why this substituter could not be reached, once the first probe found
    /// out. Every later probe is a miss that sends nothing, as nix disables a
    /// binary cache it cannot reach and goes on.
    down: OnceLock<String>,
}

#[derive(Debug)]
enum Location {
    Http(String),
    File(PathBuf),
}

/// One substituter's answer for one path.
enum Probe {
    Hit,
    Miss,
    /// This path got a wrong answer (a server error, a bad narinfo).
    Fail(String),
    /// No answer from the substituter at all: it cannot be reached. `Substituters::probe`
    /// records it and returns `Probe::Miss`, so no other code sees this.
    Down(String),
}

impl Substituters {
    /// Builds the set from substituter URLs and `name:base64` public keys.
    ///
    /// A URL repeated later in the list is dropped, and its query string
    /// (`?priority=40`) ignored. A URL with a scheme that cannot be asked
    /// (`s3://`, `ssh-ng://`) is left out of [`Substituters::urls`] and named
    /// in [`Substituters::warnings`].
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Key`] for a key that does not parse, and
    /// [`CacheError::Client`] when the HTTP client cannot start.
    pub fn new(urls: &[String], keys: &[String], require_sigs: bool) -> Result<Self, CacheError> {
        let mut list: Vec<Entry> = Vec::new();
        let mut unsupported: Vec<String> = Vec::new();
        for url in urls {
            if list.iter().any(|entry| &entry.url == url) || unsupported.contains(url) {
                continue;
            }
            match Location::parse(url) {
                Some(location) => list.push(Entry {
                    url: url.clone(),
                    location,
                    down: OnceLock::new(),
                }),
                None => unsupported.push(url.clone()),
            }
        }
        let keys = keys
            .iter()
            .map(|key| {
                TrustedPublicKey::parse(key).map_err(|err| CacheError::Key {
                    key: key.clone(),
                    reason: err.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let client = reqwest::Client::builder()
            .user_agent(concat!("nix-why-build/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .build()?;
        Ok(Self {
            list,
            unsupported,
            keys,
            require_sigs,
            concurrency: 256,
            patience: std::time::Duration::from_millis(1500),
            client,
        })
    }

    /// Caps how many paths are asked about at once (default 256, minimum 1).
    #[must_use]
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// How long one narinfo request may take before it is sent again (default
    /// 1.5 s). The retry gets eight times as long; a second stall counts as
    /// unreachable and disables the substituter. Rare stalled requests
    /// otherwise set the pace of a whole walk level.
    #[must_use]
    pub fn with_patience(mut self, patience: std::time::Duration) -> Self {
        self.patience = patience;
        self
    }

    /// The substituter URLs that will be asked, in order, duplicates dropped.
    #[must_use]
    pub fn urls(&self) -> Vec<&str> {
        self.list.iter().map(|entry| entry.url.as_str()).collect()
    }

    /// What went wrong with the substituters, each once: the ones left out
    /// for an unsupported scheme, then the ones disabled as unreachable.
    /// Complete only after the queries it concerns have finished.
    #[must_use]
    pub fn warnings(&self) -> Vec<String> {
        let unsupported = self
            .unsupported
            .iter()
            .map(|url| format!("{url}: unsupported scheme, not asked"));
        let down = self.list.iter().filter_map(|entry| {
            let why = entry.down.get()?;
            Some(format!(
                "{}: unreachable, not asked again: {why}",
                entry.url
            ))
        });
        unsupported.chain(down).collect()
    }

    /// Asks every substituter about every path, many paths at once.
    pub async fn query(&self, paths: &[StorePath]) -> HashMap<StorePath, Availability> {
        use futures::StreamExt;
        futures::stream::iter(paths.iter().cloned())
            .map(|path| async move {
                let availability = self.query_one(&path).await;
                (path, availability)
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await
    }

    /// Asks the first substituter alone, since it serves most paths; on a
    /// miss asks the rest at once. The earliest configured hit wins.
    async fn query_one(&self, path: &StorePath) -> Availability {
        let Some((first, rest)) = self.list.split_first() else {
            return Availability::Missing;
        };
        let mut probes = vec![(&first.url, self.probe(first, path).await)];
        if !matches!(probes.first(), Some((_, Probe::Hit))) {
            let later = rest
                .iter()
                .map(|entry| async move { (&entry.url, self.probe(entry, path).await) });
            probes.extend(futures::future::join_all(later).await);
        }
        let mut failure = None;
        for (url, probe) in probes {
            match probe {
                Probe::Hit => {
                    return Availability::Served {
                        substituter: url.clone(),
                    };
                }
                Probe::Miss | Probe::Down(_) => {}
                Probe::Fail(reason) => {
                    failure.get_or_insert_with(|| format!("{url}: {reason}"));
                }
            }
        }
        failure.map_or(Availability::Missing, |reason| Availability::Unknown {
            reason,
        })
    }

    /// A miss for a substituter that is down, with no request. The first
    /// request that finds one unreachable marks it down, and counts as a miss.
    async fn probe(&self, entry: &Entry, path: &StorePath) -> Probe {
        if entry.down.get().is_some() {
            return Probe::Miss;
        }
        let file = format!(
            "{}.narinfo",
            nix_derivation::nixbase32::encode(path.digest())
        );
        let body = match &entry.location {
            Location::Http(base) => self.fetch(&format!("{base}/{file}")).await,
            Location::File(dir) => read_file(&dir.join(file)).await,
        };
        match body {
            Ok(text) => self.accept(&text, path),
            Err(Probe::Down(why)) => {
                // Probes in flight at once all fail alike; the first reason stays.
                let _ = entry.down.set(why);
                Probe::Miss
            }
            Err(probe) => probe,
        }
    }

    async fn fetch(&self, url: &str) -> Result<String, Probe> {
        for patience in [self.patience, self.patience * 8] {
            if let Ok(answer) = tokio::time::timeout(patience, self.fetch_once(url)).await {
                return answer;
            }
        }
        Err(Probe::Down("timed out twice".to_owned()))
    }

    async fn fetch_once(&self, url: &str) -> Result<String, Probe> {
        let response = self.client.get(url).send().await.map_err(|err| {
            // Refused, unresolvable and connect-timed-out all land here.
            if err.is_connect() {
                Probe::Down(chain(err))
            } else {
                Probe::Fail(err.to_string())
            }
        })?;
        let status = response.status();
        if matches!(status.as_u16(), 403 | 404 | 410) {
            return Err(Probe::Miss);
        }
        if !status.is_success() {
            return Err(Probe::Fail(format!("HTTP {status}")));
        }
        response
            .text()
            .await
            .map_err(|err| Probe::Fail(err.to_string()))
    }

    fn accept(&self, text: &str, path: &StorePath) -> Probe {
        let narinfo = match text.parse::<NarInfo>() {
            Ok(narinfo) => narinfo,
            Err(err) => return Probe::Fail(format!("bad narinfo: {err}")),
        };
        if narinfo.store_path() != path {
            return Probe::Fail(format!(
                "narinfo is for {}",
                narinfo.store_path().to_basename()
            ));
        }
        if !self.require_sigs || narinfo.has_valid_signature(&StoreDir::default(), &self.keys) {
            Probe::Hit
        } else {
            Probe::Miss
        }
    }
}

impl crate::walk::Query for Substituters {
    fn query(
        &self,
        paths: &[StorePath],
    ) -> impl std::future::Future<Output = HashMap<StorePath, Availability>> + Send {
        Substituters::query(self, paths)
    }
}

impl Location {
    /// Where to ask, `None` for a scheme this crate cannot speak.
    fn parse(url: &str) -> Option<Self> {
        let base = url.split('?').next().unwrap_or(url).trim_end_matches('/');
        if base.starts_with("http://") || base.starts_with("https://") {
            Some(Self::Http(base.to_owned()))
        } else {
            base.strip_prefix("file://")
                .map(|dir| Self::File(PathBuf::from(dir)))
        }
    }
}

/// An error and its causes on one line, without the request URL, which the
/// warning names already: `error sending request: client error (Connect): ...`.
fn chain(err: reqwest::Error) -> String {
    let err = err.without_url();
    std::iter::successors(Some(&err as &dyn std::error::Error), |cause| cause.source())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

async fn read_file(path: &std::path::Path) -> Result<String, Probe> {
    match tokio::fs::read_to_string(path).await {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(Probe::Miss),
        Err(err) => Err(Probe::Fail(err.to_string())),
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
