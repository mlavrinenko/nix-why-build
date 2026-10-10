//! A fake store directory of hand-written `ATerm` `.drv` files, for tests.
#![allow(dead_code)]

use std::cell::Cell;
use std::fmt::Write as _;
use std::path::Path;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer as _, SigningKey};
use nix_derivation::{StoreDir, StorePath, nixbase32};
use nix_narinfo::NarInfo;

/// One derivation to write. Built with [`spec`], then refined field by field.
#[derive(Clone, Debug, Default)]
pub struct DrvSpec {
    pub name: String,
    pub outputs: Vec<String>,
    pub inputs: Vec<(StorePath, Vec<String>)>,
    pub env: Vec<(String, String)>,
    pub fixed: bool,
}

/// A derivation named `name` with one `out` output.
pub fn spec(name: &str) -> DrvSpec {
    DrvSpec {
        name: name.to_owned(),
        outputs: vec!["out".to_owned()],
        ..DrvSpec::default()
    }
}

impl DrvSpec {
    pub fn outputs(mut self, outputs: &[&str]) -> Self {
        self.outputs = outputs.iter().map(|&output| output.to_owned()).collect();
        self
    }

    pub fn input(mut self, drv: &StorePath, outputs: &[&str]) -> Self {
        let outputs = outputs.iter().map(|&output| output.to_owned()).collect();
        self.inputs.push((drv.clone(), outputs));
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_owned(), value.to_owned()));
        self
    }

    pub fn fixed(mut self) -> Self {
        self.fixed = true;
        self
    }
}

/// A temporary store directory; each [`FakeStore::add`] writes one `.drv`.
pub struct FakeStore {
    dir: tempfile::TempDir,
    seed: Cell<u32>,
}

impl FakeStore {
    pub fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
            seed: Cell::new(0),
        }
    }

    pub fn dir(&self) -> &Path {
        self.dir.path()
    }

    fn next_digest(&self) -> [u8; 20] {
        let seed = self.seed.get() + 1;
        self.seed.set(seed);
        let mut digest = [0_u8; 20];
        for (slot, byte) in digest.iter_mut().zip(seed.to_le_bytes()) {
            *slot = byte;
        }
        digest
    }

    /// Writes `spec` as a `.drv` file and returns its store path.
    pub fn add(&self, spec: &DrvSpec) -> StorePath {
        let drv = StorePath::from_parts(self.next_digest(), &format!("{}.drv", spec.name))
            .expect("valid drv path");
        std::fs::write(self.dir().join(drv.to_basename()), self.aterm(spec)).expect("write drv");
        drv
    }

    /// Writes raw bytes under a `.drv` name, for parse failures.
    pub fn add_raw(&self, name: &str, bytes: &[u8]) -> StorePath {
        let drv = StorePath::from_parts(self.next_digest(), &format!("{name}.drv"))
            .expect("valid drv path");
        std::fs::write(self.dir().join(drv.to_basename()), bytes).expect("write drv");
        drv
    }

    /// The output path `add` gave `output` of the derivation at `drv`.
    pub fn output(&self, drv: &StorePath, output: &str) -> StorePath {
        let bytes = std::fs::read(self.dir().join(drv.to_basename())).expect("read drv");
        let name = drv.name().strip_suffix(".drv").expect("a drv name");
        nix_derivation::Derivation::from_aterm_bytes(&bytes, name)
            .expect("parse drv")
            .resolved_outputs()
            .expect("output paths")
            .remove(output)
            .and_then(|out| out.path)
            .expect("output exists")
    }

    fn aterm(&self, spec: &DrvSpec) -> String {
        let mut env = spec.env.clone();
        env.push(("name".to_owned(), spec.name.clone()));
        let mut outputs = String::new();
        for output in &spec.outputs {
            let suffix = if output == "out" {
                String::new()
            } else {
                format!("-{output}")
            };
            let path = StorePath::from_parts(self.next_digest(), &format!("{}{suffix}", spec.name))
                .expect("valid output path")
                .to_absolute_path();
            let (algo, hash) = if spec.fixed {
                ("sha256", "0".repeat(64))
            } else {
                ("", String::new())
            };
            push_sep(&mut outputs);
            let _ = write!(
                outputs,
                "({},{},{},{})",
                quote(output),
                quote(&path),
                quote(algo),
                quote(&hash)
            );
            env.push((output.clone(), path));
        }
        let mut inputs = String::new();
        let mut sorted = spec.inputs.clone();
        sorted.sort_by_key(|(drv, _)| drv.to_absolute_path());
        for (drv, wanted) in &sorted {
            let names: Vec<String> = wanted.iter().map(|name| quote(name)).collect();
            push_sep(&mut inputs);
            let _ = write!(
                inputs,
                "({},[{}])",
                quote(&drv.to_absolute_path()),
                names.join(",")
            );
        }
        env.sort();
        let pairs: Vec<String> = env
            .iter()
            .map(|(key, value)| format!("({},{})", quote(key), quote(value)))
            .collect();
        format!(
            "Derive([{outputs}],[{inputs}],[],\"x86_64-linux\",\"/bin/sh\",[],[{}])",
            pairs.join(",")
        )
    }
}

fn push_sep(out: &mut String) {
    if !out.is_empty() {
        out.push(',');
    }
}

fn quote(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

/// A store path that no fake store wrote, for a dangling reference.
pub fn absent_drv(name: &str) -> StorePath {
    StorePath::from_parts([0xee; 20], &format!("{name}.drv")).expect("valid drv path")
}

/// Signs narinfos in a test cache.
pub const SECRET: &str = "test-cache-1:2EJ0WJNsrKIJRZSuip51j78aJfN8nPVFj/geAmnSeaa1qx20A+Oh9p6Lwvk6VlxBeipwM3pm5VprKHOI+OzCTA==";
/// Verifies [`SECRET`]'s signatures.
pub const PUBLIC: &str = "test-cache-1:tasdtAPjofaei8L5OlZcQXoqcDN6ZuVaayhziPjswkw=";

/// A narinfo for `path`, signed with `secret` when one is given.
pub fn narinfo(path: &StorePath, secret: Option<&str>) -> String {
    let text = format!(
        "StorePath: {}\nURL: nar/x.nar.xz\nCompression: xz\nNarHash: sha256:{}\nNarSize: 42\nReferences: \n",
        path.to_absolute_path(),
        nixbase32::encode(&[7; 32]),
    );
    let Some(secret) = secret else {
        return text;
    };
    let info: NarInfo = text.parse().expect("valid narinfo");
    let fingerprint = info.fingerprint(&StoreDir::default());
    let (name, keypair) = secret.split_once(':').expect("name:base64 secret");
    let keypair: [u8; 64] = STANDARD
        .decode(keypair)
        .expect("base64 secret")
        .try_into()
        .expect("64-byte keypair");
    let signer = SigningKey::from_keypair_bytes(&keypair).expect("valid keypair");
    let signature = signer.sign(fingerprint.as_bytes()).to_bytes();
    format!("{text}Sig: {name}:{}\n", STANDARD.encode(signature))
}

pub fn narinfo_name(path: &StorePath) -> String {
    format!("{}.narinfo", nixbase32::encode(path.digest()))
}

pub fn file_cache(dir: &Path, entries: &[(&StorePath, String)]) -> String {
    for (path, body) in entries {
        std::fs::write(dir.join(narinfo_name(path)), body).expect("write narinfo");
    }
    format!("file://{}", dir.display())
}

/// Serves exactly the given paths, fails on the `unknown` ones, misses the
/// rest, and records what it was asked.
#[derive(Default)]
pub struct FakeCache {
    pub served: Vec<StorePath>,
    pub unknown: Vec<StorePath>,
    pub asked: std::sync::Mutex<Vec<StorePath>>,
}

impl nix_why_build::walk::Query for FakeCache {
    async fn query(
        &self,
        paths: &[StorePath],
    ) -> std::collections::HashMap<StorePath, nix_why_build::cache::Availability> {
        use nix_why_build::cache::Availability;
        self.asked
            .lock()
            .expect("lock")
            .extend(paths.iter().cloned());
        paths
            .iter()
            .map(|path| {
                let answer = if self.served.contains(path) {
                    Availability::Served {
                        substituter: "fake".to_owned(),
                    }
                } else if self.unknown.contains(path) {
                    Availability::Unknown {
                        reason: "down".to_owned(),
                    }
                } else {
                    Availability::Missing
                };
                (path.clone(), answer)
            })
            .collect()
    }
}
