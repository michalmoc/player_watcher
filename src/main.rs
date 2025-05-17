mod constants;
mod daemon;
mod follow;
mod get;

use crate::daemon::Daemon;
use crate::follow::follow_changes;
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
        Command::Get => {
            todo!()
        }
        Command::Follow => follow_changes().await?,
        Command::Shift => {
            todo!()
        }
        Command::Unshift => {
            todo!()
        }
    }

    Ok(())
}
