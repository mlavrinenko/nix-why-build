//! Read one `.drv` file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nix_derivation::{Derivation, StorePath};

/// What the walk needs from one derivation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drv {
    /// The `.drv` store path.
    pub path: StorePath,
    /// The derivation's `name`.
    pub name: String,
    /// `pname`, from `env` or structured attrs; config glue has none.
    pub pname: Option<String>,
    /// `version`, from `env` or structured attrs.
    pub version: Option<String>,
    /// The platform it builds on, like `x86_64-linux`.
    pub system: String,
    /// Whether it is a fixed-output derivation, a fetch.
    pub fixed_output: bool,
    /// Output name to output path.
    pub outputs: BTreeMap<String, StorePath>,
    /// Input `.drv` to the outputs of it this derivation uses.
    pub inputs: BTreeMap<StorePath, BTreeSet<String>>,
}

/// Why a `.drv` could not be read.
#[derive(Debug, thiserror::Error)]
pub enum DrvError {
    /// The file is not there or not readable.
    #[error("read {path}: {source}")]
    Read {
        /// The file.
        path: String,
        /// The I/O error.
        source: std::io::Error,
    },
    /// The file is not a derivation `nix-derivation` accepts, or has an output whose path is unknown before the build.
    #[error("parse {path}: {reason}")]
    Parse {
        /// The file.
        path: String,
        /// The parser's complaint.
        reason: String,
    },
}

/// Reads `path` from `store_dir` (usually `/nix/store`).
///
/// # Errors
///
/// [`DrvError::Read`] when the file cannot be read, [`DrvError::Parse`] when
/// it is not a derivation with every output path known.
pub fn read(store_dir: &Path, path: &StorePath) -> Result<Drv, DrvError> {
    let parsed = parse(store_dir, path)?;
    let env = environment(&parsed.drv);
    let structured = parsed.drv.structured_attrs();
    let attr = |key: &str| {
        env.get(key).cloned().or_else(|| {
            structured
                .and_then(|json| json.get(key))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
    };
    Ok(Drv {
        name: parsed.drv.name().to_owned(),
        pname: attr("pname"),
        version: attr("version"),
        system: parsed.drv.system().to_owned(),
        fixed_output: parsed.fixed_output,
        outputs: parsed.outputs,
        inputs: parsed
            .drv
            .input_derivations()
            .iter()
            .map(|(drv, wanted)| (drv.clone(), wanted.outputs().clone()))
            .collect(),
        path: path.clone(),
    })
}

/// Everything a diff compares: the environment (structured attrs stay in
/// `__json`) and the input derivations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attrs {
    /// Environment variable to value.
    pub env: BTreeMap<String, String>,
    /// Input `.drv` paths.
    pub inputs: BTreeSet<StorePath>,
}

/// Reads the attributes of `path` in full, for a diff.
///
/// # Errors
///
/// As [`read`].
pub fn read_attrs(store_dir: &Path, path: &StorePath) -> Result<Attrs, DrvError> {
    let parsed = parse(store_dir, path)?;
    Ok(Attrs {
        env: environment(&parsed.drv),
        inputs: parsed.drv.input_derivations().keys().cloned().collect(),
    })
}

/// A parsed derivation whose every output path is known.
struct Parsed {
    drv: Derivation,
    outputs: BTreeMap<String, StorePath>,
    fixed_output: bool,
}

fn parse(store_dir: &Path, path: &StorePath) -> Result<Parsed, DrvError> {
    let file = store_dir.join(path.to_basename());
    let invalid = |reason: String| DrvError::Parse {
        path: file.display().to_string(),
        reason,
    };
    let bytes = std::fs::read(&file).map_err(|source| DrvError::Read {
        path: file.display().to_string(),
        source,
    })?;
    let name = path.name().strip_suffix(".drv").unwrap_or(path.name());
    let drv = Derivation::from_aterm_bytes(&bytes, name).map_err(|err| invalid(err.to_string()))?;
    let fixed_output = drv
        .is_fixed_output()
        .map_err(|err| invalid(err.to_string()))?;
    let outputs = drv
        .resolved_outputs()
        .map_err(|err| invalid(err.to_string()))?
        .into_iter()
        .map(|(output, resolved)| match resolved.path {
            Some(out) => Ok((output, out)),
            None => Err(invalid(format!(
                "output {output} has no path before it is built"
            ))),
        })
        .collect::<Result<_, _>>()?;
    Ok(Parsed {
        drv,
        outputs,
        fixed_output,
    })
}

/// The environment as nix hands it to the builder, less the output paths,
/// with structured attrs back under `__json`.
fn environment(drv: &Derivation) -> BTreeMap<String, String> {
    let lossy = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    drv.environment()
        .iter()
        .filter(|(key, _)| !drv.outputs().contains_key(*key))
        .map(|(key, value)| (key.clone(), lossy(value)))
        .chain(
            drv.structured_attrs()
                .map(|json| ("__json".to_owned(), lossy(json.raw_json()))),
        )
        .collect()
}

#[cfg(test)]
#[path = "drv_tests.rs"]
mod tests;
