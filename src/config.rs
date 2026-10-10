//! The substituters and keys nix itself would use, from `nix config show`.

use serde::Deserialize;

/// What `nix config show --json` says about substitution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NixConfig {
    /// `substituters`, with `extra-substituters` already merged in by nix.
    pub substituters: Vec<String>,
    /// `trusted-public-keys`.
    pub trusted_keys: Vec<String>,
    /// `require-sigs`.
    pub require_sigs: bool,
}

#[derive(Deserialize)]
struct Setting<T> {
    value: T,
}

#[derive(Deserialize)]
struct Raw {
    substituters: Option<Setting<Vec<String>>>,
    #[serde(rename = "trusted-public-keys")]
    trusted_public_keys: Option<Setting<Vec<String>>>,
    #[serde(rename = "require-sigs")]
    require_sigs: Option<Setting<bool>>,
}

impl NixConfig {
    /// Parses the JSON `nix config show --json` prints. A missing setting
    /// takes nix's default: no substituters, no keys, signatures required.
    ///
    /// # Errors
    ///
    /// Returns the JSON error when the text is not that shape.
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        let raw: Raw = serde_json::from_str(json)?;
        Ok(Self {
            substituters: raw.substituters.map(|s| s.value).unwrap_or_default(),
            trusted_keys: raw.trusted_public_keys.map(|s| s.value).unwrap_or_default(),
            require_sigs: raw.require_sigs.is_none_or(|s| s.value),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::NixConfig;

    #[test]
    fn reads_substituters_keys_and_require_sigs() {
        let json = r#"{
            "substituters": {"value": ["https://cache.nixos.org/", "https://x.cachix.org"], "description": "..."},
            "trusted-public-keys": {"value": ["cache.nixos.org-1:6NCH="]},
            "require-sigs": {"value": false},
            "cores": {"value": 0}
        }"#;
        assert_eq!(
            NixConfig::parse(json).expect("valid"),
            NixConfig {
                substituters: vec![
                    "https://cache.nixos.org/".to_owned(),
                    "https://x.cachix.org".to_owned()
                ],
                trusted_keys: vec!["cache.nixos.org-1:6NCH=".to_owned()],
                require_sigs: false,
            }
        );
    }

    #[test]
    fn missing_settings_take_nix_defaults() {
        assert_eq!(
            NixConfig::parse("{}").expect("valid"),
            NixConfig {
                require_sigs: true,
                ..NixConfig::default()
            }
        );
    }

    #[test]
    fn refuses_other_json() {
        assert!(NixConfig::parse("[]").is_err());
    }
}
