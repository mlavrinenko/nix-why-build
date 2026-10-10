//! Ask binary caches whether they serve a store path.
//!
//! A path counts as served when a substituter returns a narinfo for it that a
//! trusted key signed (or any narinfo, with `require-sigs` off): the same test
//! nix applies before it substitutes instead of building.

use std::collections::HashMap;
use std::path::PathBuf;

use nix_compat::store_path::StorePath;
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
    list: Vec<(String, Location)>,
    keys: Vec<nix_compat::narinfo::VerifyingKey>,
    require_sigs: bool,
    concurrency: usize,
    client: reqwest::Client,
}

#[derive(Debug)]
enum Location {
    Http(String),
    File(PathBuf),
    Unsupported,
}

/// One substituter's answer for one path.
enum Probe {
    Hit,
    Miss,
    Fail(String),
}

impl Substituters {
    /// Builds the set from substituter URLs and `name:base64` public keys.
    ///
    /// A URL repeated later in the list is dropped, and its query string
    /// (`?priority=40`) ignored.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Key`] for a key that does not parse, and
    /// [`CacheError::Client`] when the HTTP client cannot start.
    pub fn new(urls: &[String], keys: &[String], require_sigs: bool) -> Result<Self, CacheError> {
        let mut list: Vec<(String, Location)> = Vec::new();
        for url in urls {
            if list.iter().all(|(seen, _)| seen != url) {
                list.push((url.clone(), Location::parse(url)));
            }
        }
        let keys = keys
            .iter()
            .map(|key| {
                nix_compat::narinfo::VerifyingKey::parse(key).map_err(|err| CacheError::Key {
                    key: key.clone(),
                    reason: err.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let client = reqwest::Client::builder()
            .user_agent(concat!("nix-why-build/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            list,
            keys,
            require_sigs,
            concurrency: 64,
            client,
        })
    }

    /// Caps how many narinfo requests run at once (default 64, minimum 1).
    #[must_use]
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// The substituter URLs that will be asked, in order, duplicates dropped.
    #[must_use]
    pub fn urls(&self) -> Vec<&str> {
        self.list.iter().map(|(url, _)| url.as_str()).collect()
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

    async fn query_one(&self, path: &StorePath) -> Availability {
        let mut failure = None;
        for (url, location) in &self.list {
            match self.probe(location, path).await {
                Probe::Hit => {
                    return Availability::Served {
                        substituter: url.clone(),
                    };
                }
                Probe::Miss => {}
                Probe::Fail(reason) => {
                    failure.get_or_insert_with(|| format!("{url}: {reason}"));
                }
            }
        }
        failure.map_or(Availability::Missing, |reason| Availability::Unknown {
            reason,
        })
    }

    async fn probe(&self, location: &Location, path: &StorePath) -> Probe {
        let file = format!("{}.narinfo", nix_compat::nixbase32::encode(path.digest()));
        let body = match location {
            Location::Http(base) => self.fetch(&format!("{base}/{file}")).await,
            Location::File(dir) => read_file(&dir.join(file)).await,
            Location::Unsupported => Err(Probe::Fail("unsupported substituter scheme".into())),
        };
        match body {
            Ok(text) => self.accept(&text, path),
            Err(probe) => probe,
        }
    }

    async fn fetch(&self, url: &str) -> Result<String, Probe> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| Probe::Fail(err.to_string()))?;
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
        let narinfo = match nix_compat::narinfo::NarInfo::parse(text) {
            Ok(narinfo) => narinfo,
            Err(err) => return Probe::Fail(format!("bad narinfo: {err}")),
        };
        if narinfo.store_path.digest() != path.digest() || narinfo.store_path.name() != path.name()
        {
            return Probe::Fail(format!("narinfo is for {}", narinfo.store_path));
        }
        if !self.require_sigs {
            return Probe::Hit;
        }
        let fingerprint = narinfo.fingerprint();
        let signed = narinfo.signatures.iter().any(|signature| {
            self.keys
                .iter()
                .any(|key| key.name() == *signature.name() && key.verify(&fingerprint, signature))
        });
        if signed { Probe::Hit } else { Probe::Miss }
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
    fn parse(url: &str) -> Self {
        let base = url.split('?').next().unwrap_or(url).trim_end_matches('/');
        if base.starts_with("http://") || base.starts_with("https://") {
            Self::Http(base.to_owned())
        } else if let Some(dir) = base.strip_prefix("file://") {
            Self::File(PathBuf::from(dir))
        } else {
            Self::Unsupported
        }
    }
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
