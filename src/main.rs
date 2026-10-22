use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "cctop",
    version,
    about = "A btop for cornercase: what every pane costs, and what fills Claude's context"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Watch every pane and notify through cornercase when one crosses a threshold")]
    Watch,
    #[command(
        about = "Open cctop in a split beside the clicked pane in cornercase, or focus the one already in that tab"
    )]
    Open,
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        None => cctop::run_tui()?,
        Some(Command::Watch) => cctop::run_watch()?,
        Some(Command::Open) => cctop::run_open()?,
    }
    Ok(())
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
