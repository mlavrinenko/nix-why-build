use anyhow::Result;
use clap::Parser;

/// Explain why a Nix build compiles what no binary cache serves
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Name to greet
    #[arg(default_value = "nix-why-build")]
    name: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    nix_why_build::greet(&cli.name)?;

    Ok(())
}
