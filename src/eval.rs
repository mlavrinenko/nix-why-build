//! Turn an installable into the `.drv` the walk starts from.

use std::process::Stdio;

use nix_derivation::StorePath;
use tokio::io::AsyncWriteExt as _;
use tokio::process::ChildStdin;

/// The attribute names of an attribute path: dots separate them, except
/// inside double quotes (`nixosConfigurations."a.b"` is two names, the second
/// keeping its quotes), as nix reads it.
fn attr_names(attr: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (at, ch) in attr.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '.' if !quoted => {
                names.push(&attr[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    names.push(&attr[start..]);
    names
}

/// What to append to a configuration attribute to get the package it builds,
/// when `attr` names a configuration.
fn build_suffix(attr: &str) -> Option<&'static str> {
    match attr_names(attr).as_slice() {
        ["nixosConfigurations", _] => Some(".config.system.build.toplevel"),
        ["homeConfigurations", _] => Some(".activationPackage"),
        ["darwinConfigurations", _] => Some(".system"),
        _ => None,
    }
}

/// The attribute a configuration installable builds: a NixOS system's
/// toplevel, a home-manager activation package, a nix-darwin system. Any
/// other installable is returned as is.
#[must_use]
pub fn toplevel_attr(installable: &str) -> String {
    let Some((flake, attr)) = installable.split_once('#') else {
        return installable.to_owned();
    };
    format!("{flake}#{attr}{}", build_suffix(attr).unwrap_or_default())
}

/// The configuration an installable names, when it names one
/// (`.#nixosConfigurations.tank`): its options can be indexed.
#[must_use]
pub fn config_attr(installable: &str) -> Option<String> {
    let (_, attr) = installable.split_once('#')?;
    build_suffix(attr).map(|_| installable.to_owned())
}

/// The flake reference of an installable: what comes before `#`, else the
/// whole installable (`.`, `github:o/r`). A store path is no flake.
#[must_use]
pub fn flake_ref(installable: &str) -> Option<&str> {
    match installable.split_once('#') {
        Some((flake, _)) => Some(flake),
        None if installable.starts_with("/nix/store/") => None,
        None => Some(installable),
    }
}

/// A `.drv` path given directly, rather than an installable to evaluate.
#[must_use]
pub fn as_drv_path(installable: &str) -> Option<StorePath> {
    if !installable.ends_with(".drv") {
        return None;
    }
    StorePath::from_absolute_path(installable.as_bytes()).ok()
}

/// Why evaluation failed.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    /// `nix` could not be started.
    #[error("run nix: {0}")]
    Spawn(#[from] std::io::Error),
    /// `nix` exited non-zero; its stderr says why.
    #[error("{command} failed:\n{stderr}")]
    Failed {
        /// The command line.
        command: String,
        /// What nix printed.
        stderr: String,
    },
    /// `nix` printed something that is not a store path.
    #[error("{command} printed {output:?}, not a store path")]
    NotAPath {
        /// The command line.
        command: String,
        /// What it printed.
        output: String,
    },
}

/// Runs `nix` with `args` and returns its stdout. It enables `nix-command`
/// and `flakes` for the call, as `nixos-rebuild --flake` does, so a nix that
/// has them off in its configuration still works.
///
/// # Errors
///
/// [`EvalError::Spawn`] when nix cannot start, [`EvalError::Failed`] when it
/// exits non-zero.
pub async fn nix(args: &[&str]) -> Result<String, EvalError> {
    run("nix", &with_features(args), "").await
}

/// Runs `nix` with `args`, writes `stdin` to it and returns its stdout. Linux
/// caps one argument at 128 KiB; stdin has no such limit.
///
/// # Errors
///
/// As [`nix`].
pub async fn nix_with_stdin(args: &[&str], stdin: &str) -> Result<String, EvalError> {
    run("nix", &with_features(args), stdin).await
}

/// `args` after the flags that enable `nix-command` and `flakes` for one call.
fn with_features<'a>(args: &[&'a str]) -> Vec<&'a str> {
    ["--extra-experimental-features", "nix-command flakes"]
        .into_iter()
        .chain(args.iter().copied())
        .collect()
}

/// Runs `program`, writing `stdin` to it as it reads. A program that exits
/// without reading all of it is judged by its status and stderr, not by the
/// broken pipe it leaves.
async fn run(program: &str, args: &[&str], stdin: &str) -> Result<String, EvalError> {
    let mut child = tokio::process::Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let pipe = child.stdin.take();
    let (fed, output) = tokio::join!(feed(pipe, stdin), child.wait_with_output());
    let output = output?;
    if !output.status.success() {
        return Err(EvalError::Failed {
            command: format!("{program} {}", args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr)
                .trim_end()
                .to_owned(),
        });
    }
    fed?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Writes `input` to `pipe` and closes it. A closed read end is not an error
/// here: the program stopped reading, and its own words say why.
async fn feed(pipe: Option<ChildStdin>, input: &str) -> std::io::Result<()> {
    let Some(mut pipe) = pipe else {
        return Ok(());
    };
    match pipe.write_all(input.as_bytes()).await {
        Err(err) if err.kind() != std::io::ErrorKind::BrokenPipe => Err(err),
        _ => Ok(()),
    }
}

/// The first line of nix's complaint that says `error`, else its first line.
#[must_use]
pub fn first_error(text: &str) -> String {
    text.lines()
        .find(|line| line.contains("error"))
        .or_else(|| text.lines().next())
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The `.drv` an installable builds: the path itself when it is one, else
/// what `nix path-info --derivation` resolves the installable to, which also
/// writes the `.drv` files. Nix parses the installable, so anything
/// `nix build` accepts works; only a configuration gets its build attribute
/// appended.
///
/// # Errors
///
/// Any [`EvalError`] from running nix.
pub async fn drv_path(installable: &str) -> Result<StorePath, EvalError> {
    if let Some(path) = as_drv_path(installable) {
        return Ok(path);
    }
    let attr = toplevel_attr(installable);
    let output = nix(&["path-info", "--derivation", &attr]).await?;
    StorePath::from_absolute_path(output.trim().as_bytes()).map_err(|_| EvalError::NotAPath {
        command: format!("nix path-info --derivation {attr}"),
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::{EvalError, as_drv_path, run, toplevel_attr};

    #[test]
    fn maps_configuration_installables_to_what_they_build() {
        let cases = [
            (
                ".#nixosConfigurations.tank",
                ".#nixosConfigurations.tank.config.system.build.toplevel",
            ),
            (
                "github:o/r#homeConfigurations.me",
                "github:o/r#homeConfigurations.me.activationPackage",
            ),
            (
                ".#darwinConfigurations.mac",
                ".#darwinConfigurations.mac.system",
            ),
            (
                ".#nixosConfigurations.tank.config.system.build.vm",
                ".#nixosConfigurations.tank.config.system.build.vm",
            ),
            (
                r#".#nixosConfigurations."host.lan""#,
                r#".#nixosConfigurations."host.lan".config.system.build.toplevel"#,
            ),
            ("nixpkgs#hello", "nixpkgs#hello"),
            ("./result", "./result"),
            (".", "."),
        ];
        for (input, expected) in cases {
            assert_eq!(toplevel_attr(input), expected, "{input}");
        }
    }

    #[test]
    fn configurations_have_options_and_flakes_have_refs() {
        assert_eq!(
            super::config_attr(".#nixosConfigurations.tank").as_deref(),
            Some(".#nixosConfigurations.tank")
        );
        assert_eq!(
            super::config_attr("github:o/r#homeConfigurations.me").as_deref(),
            Some("github:o/r#homeConfigurations.me")
        );
        assert_eq!(
            super::config_attr(".#darwinConfigurations.mac").as_deref(),
            Some(".#darwinConfigurations.mac")
        );
        assert_eq!(
            super::config_attr(".#nixosConfigurations.tank.config.system.build.vm"),
            None
        );
        assert_eq!(super::config_attr("nixpkgs#hello"), None);
        assert_eq!(super::config_attr("/nix/store/x.drv"), None);
        assert_eq!(
            super::flake_ref("github:o/r#homeConfigurations.me"),
            Some("github:o/r")
        );
        assert_eq!(super::flake_ref(".#x"), Some("."));
    }

    #[test]
    fn a_quoted_name_holds_its_dots() {
        let quoted = r#".#nixosConfigurations."host.lan""#;
        assert_eq!(super::config_attr(quoted).as_deref(), Some(quoted));
        assert_eq!(
            super::config_attr(r#".#nixosConfigurations."host.lan".config"#),
            None
        );
        assert_eq!(super::config_attr(r#".#packages."a.b".default"#), None);
    }

    #[test]
    fn a_flake_installable_is_its_own_ref_unless_it_is_a_store_path() {
        assert_eq!(super::flake_ref("."), Some("."));
        assert_eq!(super::flake_ref("github:o/r"), Some("github:o/r"));
        assert_eq!(super::flake_ref("nixpkgs"), Some("nixpkgs"));
        assert_eq!(super::flake_ref("/nix/store/x.drv"), None);
        assert_eq!(super::flake_ref("/nix/store/x-hello"), None);
    }

    #[test]
    fn takes_a_drv_path_as_is() {
        let path = "/nix/store/rcfnhh5dg3n04xibyjgv5q84yps9ifcr-nixos-system-tank.drv";
        assert_eq!(
            as_drv_path(path).map(|p| p.to_absolute_path()).as_deref(),
            Some(path)
        );
        assert_eq!(
            as_drv_path("/nix/store/rcfnhh5dg3n04xibyjgv5q84yps9ifcr-hello"),
            None
        );
        assert_eq!(as_drv_path("nixpkgs#foo.drv"), None);
    }

    #[tokio::test]
    async fn a_program_gets_all_of_its_stdin_whatever_the_size() {
        let input = "x".repeat(3 << 20);
        let echoed = run("cat", &[], &input).await.expect("cat succeeds");
        assert!(
            echoed == input,
            "{} bytes back for {}",
            echoed.len(),
            input.len()
        );
        assert_eq!(run("cat", &[], "").await.expect("cat succeeds"), "");
    }

    #[tokio::test]
    async fn a_program_that_stops_reading_is_judged_by_its_exit() {
        let input = "x".repeat(3 << 20);
        let err = run("sh", &["-c", "echo boom >&2; exit 3"], &input)
            .await
            .expect_err("a non-zero exit fails");
        assert!(
            matches!(&err, EvalError::Failed { stderr, .. } if stderr == "boom"),
            "{err:?}"
        );
        let said = run("sh", &["-c", "echo ok"], &input).await;
        assert_eq!(said.expect("an exit of 0 succeeds"), "ok\n");
    }
}
