use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use engine::{config::SimulationConfig, environment::Environment, simulation::Simulation};

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
    ///
    /// Every knob has a default, so `schism run` works with no arguments.
    /// Overrides layer in order: defaults -> `--config` file -> individual flags.
    Run {
        /// Load a full or partial config from a JSON file (same shape as
        /// `SimulationConfig`). Anything omitted keeps its default.
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Override the starting environment.
        #[arg(short, long)]
        environment: Option<Environment>,

        /// Override the number of generations to run.
        #[arg(short, long)]
        num_generations: Option<u32>,

        /// Override the starting population size.
        #[arg(short = 'p', long)]
        starting_population: Option<u32>,

        /// Override the rng seed.
        #[arg(short, long)]
        seed: Option<u64>,
    },
}

impl Cli {
    /// Parse arguments from the environment and run the CLI.
    pub fn run() -> Result<()> {
        let cli = Cli::parse();
        match cli.command {
            Command::Run {
                config,
                environment,
                num_generations,
                starting_population,
                seed,
            } => {
                // precedence: defaults -> config file -> individual flags
                let mut simulation_config = match config {
                    Some(path) => {
                        let contents = std::fs::read_to_string(&path)
                            .with_context(|| format!("reading config file {}", path.display()))?;
                        serde_json::from_str(&contents)
                            .with_context(|| format!("parsing config file {}", path.display()))?
                    }
                    None => SimulationConfig::default(),
                };

                if let Some(environment) = environment {
                    simulation_config.world.environment = environment;
                }
                if let Some(num_generations) = num_generations {
                    simulation_config.world.num_generations = num_generations;
                }
                if let Some(starting_population) = starting_population {
                    simulation_config.world.starting_population = starting_population;
                }
                if let Some(seed) = seed {
                    simulation_config.world.seed = Some(seed);
                }

                simulation_config
                    .validate()
                    .context("invalid simulation config")?;

                let mut sim = Simulation::new(simulation_config)?;
                sim.run()?;
            }
        }

        Ok(())
    }
}
