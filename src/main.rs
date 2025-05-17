mod constants;
mod daemon;
mod follow;
mod get;
mod shift;

use crate::daemon::Daemon;
use crate::follow::follow_changes;
use crate::get::get_active_player;
use crate::shift::{next_player, previous_player};
use clap::{Parser, Subcommand};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Subcommand)]
enum Command {
    Daemon,
    Get,
    Follow,
    Shift,
    Unshift,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Command::Daemon => Daemon::new().await?.run().await?,
        Command::Get => get_active_player().await?,
        Command::Follow => follow_changes().await?,
        Command::Shift => next_player().await?,
        Command::Unshift => previous_player().await?,
    }

    Ok(())
}
