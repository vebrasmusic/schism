use clap::ValueEnum;
use strum_macros::Display;

/// defines the differnet env types we can start in
#[derive(Debug, Clone, ValueEnum, Display)]
pub enum Environment {
    Desert,
    Jungle,
    Plains,
}
