//! A fake store directory of hand-written `ATerm` `.drv` files, for tests.

use std::cell::Cell;
use std::fmt::Write as _;
use std::path::Path;

use nix_compat::store_path::StorePath;

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
        let drv = StorePath::from_name_and_digest_fixed(
            &format!("{}.drv", spec.name),
            self.next_digest(),
        )
        .expect("valid drv path");
        std::fs::write(self.dir().join(drv.to_string()), self.aterm(spec)).expect("write drv");
        drv
    }

    /// Writes raw bytes under a `.drv` name, for parse failures.
    pub fn add_raw(&self, name: &str, bytes: &[u8]) -> StorePath {
        let drv = StorePath::from_name_and_digest_fixed(&format!("{name}.drv"), self.next_digest())
            .expect("valid drv path");
        std::fs::write(self.dir().join(drv.to_string()), bytes).expect("write drv");
        drv
    }

    /// The output path `add` gave `output` of the derivation at `drv`.
    pub fn output(&self, drv: &StorePath, output: &str) -> StorePath {
        let bytes = std::fs::read(self.dir().join(drv.to_string())).expect("read drv");
        let parsed = nix_compat::derivation::UnverifiedDerivation::from_aterm_bytes(&bytes)
            .expect("parse drv");
        parsed
            .outputs()
            .iter()
            .find(|(name, _)| name.as_str() == output)
            .map(|(_, path)| path.clone())
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
            let path = StorePath::from_name_and_digest_fixed(
                &format!("{}{suffix}", spec.name),
                self.next_digest(),
            )
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
    StorePath::from_name_and_digest_fixed(&format!("{name}.drv"), [0xee; 20])
        .expect("valid drv path")
}
