use std::io::{ErrorKind, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Explain what a Nix build would compile because no binary cache serves it
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Flake installable (`.#nixosConfigurations.tank`) or a `.drv` path
    installable: String,
    /// Ask this substituter instead of nix's own list (repeatable)
    #[arg(long = "substituter", value_name = "URL")]
    substituters: Vec<String>,
    /// Trust this `name:key` instead of nix's own list (repeatable)
    #[arg(long = "trusted-key", value_name = "KEY")]
    trusted_keys: Vec<String>,
    /// Print the report as JSON
    #[arg(long)]
    json: bool,
    /// Where `.drv` files live
    #[arg(long, value_name = "DIR", default_value = "/nix/store", hide = true)]
    store_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let mut options = nix_why_build::Options::new(&cli.installable).with_store_dir(cli.store_dir);
    if !cli.substituters.is_empty() {
        options = options.with_substituters(cli.substituters);
    }
    if !cli.trusted_keys.is_empty() {
        options = options.with_trusted_keys(cli.trusted_keys);
    }
    let report = nix_why_build::explain(&options).await?;
    let written = if cli.json {
        writeln!(
            std::io::stdout(),
            "{}",
            serde_json::to_string_pretty(&report)?
        )
    } else {
        write!(std::io::stdout(), "{}", report.human())
    };
    match written {
        // `| head` and a pager quit early close stdout; that is no failure.
        Err(err) if err.kind() == ErrorKind::BrokenPipe => return Ok(()),
        other => other?,
    }
    if !cli.json {
        write!(std::io::stderr(), "{}", report.summary())?;
    }
    Ok(())
}
