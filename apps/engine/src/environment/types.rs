use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::Display;

/// defines the differnet env types we can start in
#[derive(Debug, Clone, Default, ValueEnum, Display, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Desert,
    Jungle,
    #[default]
    Plains,
}

/// per-environment tunables. resolved from the chosen `Environment` at
/// `Simulation::new` time rather than loaded from JSON — it's picked by the
/// `--environment` CLI flag, not hand-edited like the other sub-configs.
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    /// carrying capacity (K): the living population the land can sustain. as the
    /// population approaches and passes this, mortality is scaled up, which bends
    /// the otherwise-exponential growth into an S-curve that settles around K.
    pub carrying_capacity: u64,
}

impl Environment {
    /// the env -> config map: resolve this environment's tunables. carrying
    /// capacity tracks how much life the land supports — barren desert holds far
    /// fewer people than fertile plains, with jungle in between (lush, but hard
    /// to clear and farm).
    pub fn config(&self) -> EnvironmentConfig {
        match self {
            Environment::Desert => EnvironmentConfig {
                carrying_capacity: 100_000,
            },
            Environment::Jungle => EnvironmentConfig {
                carrying_capacity: 1_000_000,
            },
            Environment::Plains => EnvironmentConfig {
                carrying_capacity: 5_000_000,
            },
        }
    }
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Environment::default().config()
    }
}
