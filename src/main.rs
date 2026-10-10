use std::io::{ErrorKind, IsTerminal, Write};
use std::path::PathBuf;

use anyhow::Result;
use clap::{ColorChoice, Parser};

mod terminal;

/// Explain what a Nix build would compile because no binary cache serves it
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Flake installable (`.#nixosConfigurations.mypc`) or a `.drv` path
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
    /// When to color the report; `auto` colors a terminal unless `NO_COLOR` is set
    #[arg(long, value_enum, value_name = "WHEN", default_value_t)]
    color: ColorChoice,
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
    // Logging at info prints the same phases with their timings, and would
    // tear the line, so a terminal gets one or the other.
    let live = std::io::stderr().is_terminal() && !log::log_enabled!(log::Level::Info);
    if live {
        options = options.with_progress(|progress| terminal::draw(&progress.to_string()));
    }
    let explained = nix_why_build::explain(&options).await;
    if live {
        terminal::clear();
    }
    let report = explained?;
    let written = if cli.json {
        writeln!(
            std::io::stdout(),
            "{}",
            serde_json::to_string_pretty(&report)?
        )
    } else {
        let stdout = std::io::stdout();
        let styled = terminal::wants_color(
            cli.color,
            std::env::var_os("NO_COLOR").as_deref(),
            stdout.is_terminal(),
        );
        let text = if styled {
            report.human_styled()
        } else {
            report.human()
        };
        write!(stdout.lock(), "{text}")
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
