//! Read one `.drv` file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nix_compat::store_path::StorePath;

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
    /// The file is not a derivation `nix-compat` accepts.
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
    let (builder, outputs) = parse(store_dir, path)?;
    let env = |key: &str| {
        builder
            .environment
            .get(key)
            .map(|value| String::from_utf8_lossy(value).into_owned())
    };
    let structured =
        env("__json").and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok());
    let attr = |key: &str| {
        env(key).or_else(|| {
            structured
                .as_ref()
                .and_then(|json| json.get(key))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
    };
    Ok(Drv {
        name: path.name().trim_end_matches(".drv").to_owned(),
        pname: attr("pname"),
        version: attr("version"),
        fixed_output: outputs.is_fixed(),
        outputs: outputs
            .iter()
            .map(|(name, out)| (name.as_str().to_owned(), out.clone()))
            .collect(),
        inputs: builder
            .input_derivations
            .iter()
            .map(|(drv, wanted)| {
                let wanted = wanted.iter().map(|name| name.as_str().to_owned()).collect();
                (drv.clone(), wanted)
            })
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
    let (builder, _) = parse(store_dir, path)?;
    Ok(Attrs {
        env: builder
            .environment
            .iter()
            .map(|(key, value)| (key.clone(), String::from_utf8_lossy(value).into_owned()))
            .collect(),
        inputs: builder.input_derivations.keys().cloned().collect(),
    })
}

fn parse(
    store_dir: &Path,
    path: &StorePath,
) -> Result<
    (
        nix_compat::derivation::DerivationBuilder,
        nix_compat::derivation::Outputs,
    ),
    DrvError,
> {
    let file = store_dir.join(path.to_string());
    let bytes = std::fs::read(&file).map_err(|source| DrvError::Read {
        path: file.display().to_string(),
        source,
    })?;
    let parsed =
        nix_compat::derivation::UnverifiedDerivation::from_aterm_bytes(&bytes).map_err(|err| {
            DrvError::Parse {
                path: file.display().to_string(),
                reason: format!("{err:?}"),
            }
        })?;
    Ok(parsed.into_parts())
}

#[cfg(test)]
#[path = "drv_tests.rs"]
mod tests;
