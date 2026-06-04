use std::fmt::format;

use clap::{Parser, Subcommand};

use crate::environment::Environment;

/// Schism engine command line interface.
///
/// Add new arguments as fields on this struct.
#[derive(Parser, Debug)]
#[command(name = "schism", version, about = "Schism engine", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Top-level commands supported by the engine.
///
/// Add new commands as variants here.
#[derive(Subcommand, Debug)]
enum Command {
    /// Run the engine.
    Run {
        #[arg(short, long)]
        environment: Environment,
    },
}

impl Cli {
    /// Parse arguments from the environment and run the CLI.
    pub fn run() {
        let cli = Cli::parse();
        match cli.command {
            Command::Run { environment } => println!("running in env {environment}"),
        }
    }
}
