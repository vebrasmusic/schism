use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum_macros::Display;

/// defines the differnet env types we can start in
#[derive(Debug, Clone, Default, ValueEnum, Display, Serialize, Deserialize)]
pub enum Environment {
    Desert,
    Jungle,
    #[default]
    Plains,
}
