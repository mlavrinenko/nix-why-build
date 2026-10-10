//! Turn an installable into the `.drv` the walk starts from.

use nix_compat::store_path::StorePath;

/// The attribute a configuration installable builds: a NixOS system's
/// toplevel, a home-manager activation package, a nix-darwin system. Any
/// other installable is returned as is.
#[must_use]
pub fn toplevel_attr(installable: &str) -> String {
    let Some((flake, attr)) = installable.split_once('#') else {
        return installable.to_owned();
    };
    let parts: Vec<&str> = attr.split('.').collect();
    let suffix = match parts.as_slice() {
        ["nixosConfigurations", _] => ".config.system.build.toplevel",
        ["homeConfigurations", _] => ".activationPackage",
        ["darwinConfigurations", _] => ".system",
        _ => "",
    };
    format!("{flake}#{attr}{suffix}")
}

/// The configuration an installable names, when it names one
/// (`.#nixosConfigurations.tank`): its options can be indexed.
#[must_use]
pub fn config_attr(installable: &str) -> Option<String> {
    let (_, attr) = installable.split_once('#')?;
    let parts: Vec<&str> = attr.split('.').collect();
    match parts.as_slice() {
        [
            "nixosConfigurations" | "homeConfigurations" | "darwinConfigurations",
            _,
        ] => Some(installable.to_owned()),
        _ => None,
    }
}

/// The flake reference of a flake installable: what comes before `#`.
#[must_use]
pub fn flake_ref(installable: &str) -> Option<&str> {
    installable.split_once('#').map(|(flake, _)| flake)
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

/// Runs `nix` with `args` and returns its stdout.
///
/// # Errors
///
/// [`EvalError::Spawn`] when nix cannot start, [`EvalError::Failed`] when it
/// exits non-zero.
pub async fn nix(args: &[&str]) -> Result<String, EvalError> {
    let output = tokio::process::Command::new("nix")
        .args(args)
        .output()
        .await?;
    if !output.status.success() {
        return Err(EvalError::Failed {
            command: format!("nix {}", args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr)
                .trim_end()
                .to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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
/// `nix eval --raw <toplevel>.drvPath`, which also writes the `.drv` files.
///
/// # Errors
///
/// Any [`EvalError`] from running nix.
pub async fn drv_path(installable: &str) -> Result<StorePath, EvalError> {
    if let Some(path) = as_drv_path(installable) {
        return Ok(path);
    }
    let attr = format!("{}.drvPath", toplevel_attr(installable));
    let output = nix(&["eval", "--raw", &attr]).await?;
    StorePath::from_absolute_path(output.trim().as_bytes()).map_err(|_| EvalError::NotAPath {
        command: format!("nix eval --raw {attr}"),
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::{as_drv_path, toplevel_attr};

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
            ("nixpkgs#hello", "nixpkgs#hello"),
            ("./result", "./result"),
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
        assert_eq!(super::flake_ref("/nix/store/x.drv"), None);
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
}
