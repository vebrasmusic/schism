mod adherent;
mod cli;
mod config;
mod environment;
mod output;
mod probability;
mod religion;
mod simulation;

use cli::Cli;

fn main() -> anyhow::Result<()> {
    Cli::run()
}
