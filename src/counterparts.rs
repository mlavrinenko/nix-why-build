//! The derivation nixpkgs builds under a root's attribute: what a cache
//! would have, and what the root is compared against.

use serde::{Deserialize, Serialize};

/// The Nix function that looks roots up, called with `--impure` since it
/// imports flake inputs by store path.
pub const COUNTERPARTS_EXPR: &str = include_str!("counterparts.nix");

/// A nixpkgs-like flake input: its label and store path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Input {
    /// `nixpkgs`, `unstable`, `home-manager/nixpkgs`.
    pub label: String,
    /// Its source in the store.
    pub path: String,
}

/// One root to look up.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Lookup {
    /// The root's `pname`, which the attributes were guessed from.
    #[serde(skip)]
    pub pname: String,
    /// Attribute paths to try, in order.
    pub attrs: Vec<Vec<String>>,
    /// The package sets to look in: empty for the top level, or an attribute
    /// like `pkgsCuda` or `pkgsi686Linux`.
    pub variants: Vec<String>,
}

impl Lookup {
    /// `pname` under its [`attr_guesses`], in `variants`.
    #[must_use]
    pub fn new(pname: &str, variants: Vec<String>) -> Self {
        Self {
            pname: pname.to_owned(),
            attrs: attr_guesses(pname),
            variants,
        }
    }
}

#[derive(Serialize)]
struct Query<'a> {
    system: &'a str,
    inputs: &'a [Input],
    roots: &'a [Lookup],
}

/// One attribute that exists in one input.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// The input's label.
    pub input: String,
    /// The attribute path, dotted.
    pub attr: String,
    /// Empty, or `pkgsCuda`.
    pub variant: String,
    /// Its `.drv`.
    pub drv: String,
    /// Its `version`.
    pub version: Option<String>,
    /// `meta.unfree`: Hydra does not build it.
    pub unfree: bool,
    /// `meta.broken`.
    pub broken: bool,
    /// `meta.hydraPlatforms`, when set.
    pub hydra_platforms: Option<Vec<String>>,
}

impl Candidate {
    /// `unstable#pkgsCuda.llama-cpp`.
    #[must_use]
    pub fn label(&self) -> String {
        let variant = if self.variant.is_empty() {
            String::new()
        } else {
            format!("{}.", self.variant)
        };
        format!("{}#{variant}{}", self.input, self.attr)
    }
}

/// An input's `hello`, the yardstick for whether a cache has caught up.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Hello {
    /// The input's label.
    pub input: String,
    /// `hello`'s `.drv`, if it evaluates.
    pub drv: Option<String>,
}

/// What the evaluation found.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Found {
    /// One `hello` per input that imported.
    pub hello: Vec<Hello>,
    /// Per lookup, in order, every candidate that exists.
    pub roots: Vec<Vec<Candidate>>,
}

/// Attribute guesses for a `pname`: itself, camelCase, and the Python and
/// Perl package sets.
#[must_use]
pub fn attr_guesses(pname: &str) -> Vec<Vec<String>> {
    let camel: String = pname
        .split('-')
        .enumerate()
        .map(|(at, part)| {
            if at == 0 {
                part.to_owned()
            } else {
                capitalize(part)
            }
        })
        .collect();
    let perl: String = pname.chars().filter(char::is_ascii_alphanumeric).collect();
    let mut guesses = vec![vec![pname.to_owned()]];
    if camel != pname {
        guesses.push(vec![camel]);
    }
    guesses.push(vec!["python3Packages".to_owned(), pname.to_owned()]);
    guesses.push(vec!["perlPackages".to_owned(), capitalize(&perl)]);
    guesses
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// The expression for `nix eval --impure --json --file -`, read from stdin.
///
/// # Errors
///
/// When the query does not serialize, which plain strings always do.
pub fn expression(
    system: &str,
    inputs: &[Input],
    roots: &[Lookup],
) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string(&Query {
        system,
        inputs,
        roots,
    })?;
    Ok(format!(
        "({COUNTERPARTS_EXPR}) (builtins.fromJSON {})",
        nix_string(&json)
    ))
}

/// What `find` runs: nix's arguments, and what it reads on stdin. Linux caps
/// each argument at 128 KiB and a query grows with the roots, so only the
/// constant part is an argument.
#[derive(Debug)]
pub(crate) struct Call {
    /// The arguments to `nix`.
    pub args: Vec<String>,
    /// The standard input to give it.
    pub stdin: String,
}

/// The call that looks every root up in every input.
///
/// # Errors
///
/// When the query does not serialize, which plain strings always do.
pub(crate) fn call(
    system: &str,
    inputs: &[Input],
    roots: &[Lookup],
) -> Result<Call, serde_json::Error> {
    Ok(Call {
        args: ["eval", "--impure", "--json", "--file", "-"]
            .map(str::to_owned)
            .to_vec(),
        stdin: expression(system, inputs, roots)?,
    })
}

/// Looks every root up in every input.
///
/// # Errors
///
/// Why nix failed, or why its output does not parse, as one line.
pub async fn find(system: &str, inputs: &[Input], roots: &[Lookup]) -> Result<Found, String> {
    let call = call(system, inputs, roots).map_err(|err| err.to_string())?;
    let args: Vec<&str> = call.args.iter().map(String::as_str).collect();
    let json = crate::eval::nix_with_stdin(&args, &call.stdin)
        .await
        .map_err(|err| crate::eval::first_error(&err.to_string()))?;
    serde_json::from_str(&json).map_err(|err| err.to_string())
}

/// `text` as a double-quoted Nix string.
pub(crate) fn nix_string(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("${", "\\${");
    format!("\"{escaped}\"")
}

#[cfg(test)]
#[path = "counterparts_tests.rs"]
mod tests;
