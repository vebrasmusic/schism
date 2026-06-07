use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::environment::Environment;

/// Central control board for every tunable in the simulation.
///
/// Built once at startup — from defaults, an optional `--config` file, and CLI
/// flags — then handed to `Simulation::new`. The engine reads from this and
/// never reaches for hardcoded values, so any future caller (an HTTP backend, a
/// test harness) just constructs this same struct and the engine is unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulationConfig {
    pub world: WorldConfig,
    pub adherent: AdherentConfig,
    pub religion: ReligionConfig,
}

/// Top-level run settings: where, how long, how big, and the rng seed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldConfig {
    /// environment the simulation starts in
    pub environment: Environment,

    /// number of generations (ticks) to run
    pub num_generations: u32,

    /// how many adherents the root religion starts with
    pub starting_population: u32,

    /// rng seed — fix this to reproduce / compare runs
    pub seed: u64,
}

/// Per-adherent lifecycle rates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdherentConfig {
    /// heterodoxy every adherent is born with
    pub starting_heterodoxy: f64,

    /// base chance to convert when a new sect appears, scaled by heterodoxy
    pub conversion_base_rate: f64,

    /// base per-tick heterodoxy drift, scaled by heterodoxy
    pub heterodoxy_change_base_rate: f64,

    /// chance of death per tick, looked up by age band
    pub mortality: Vec<AgeBand>,

    /// chance of giving birth per tick, looked up by age band
    pub birth: Vec<AgeBand>,
}

/// Knobs governing religions and when they split.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReligionConfig {
    /// minimum living congregation before a religion can schism at all
    pub min_congregation: usize,

    /// heterodoxy strictly above this counts toward the "high heterodoxy" share
    pub high_heterodoxy_threshold: f64,

    /// congregation size at which the population factor saturates to 1.0
    pub population_factor_pivot: f64,

    /// base multiplier on the per-tick schism chance
    pub schism_base_rate: f64,
}

/// A single age bracket: applies `rate` to everyone with `age <= max_age`,
/// taking the first matching band. The last band should use `u8::MAX` so it
/// acts as a catch-all — `validate` enforces this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeBand {
    pub max_age: u8,
    pub rate: f64,
}

impl AdherentConfig {
    /// mortality rate for an adherent of the given age
    pub fn mortality_rate(&self, age: u8) -> f64 {
        Self::lookup(&self.mortality, age)
    }

    /// birth rate for an adherent of the given age
    pub fn birth_rate(&self, age: u8) -> f64 {
        Self::lookup(&self.birth, age)
    }

    /// rate from the first band covering `age`. bands are validated to end in a
    /// `u8::MAX` catch-all, so a match always exists; the 0.0 fallback only
    /// fires if that invariant is somehow broken.
    fn lookup(bands: &[AgeBand], age: u8) -> f64 {
        bands
            .iter()
            .find(|band| age <= band.max_age)
            .map(|band| band.rate)
            .unwrap_or(0.0)
    }
}

/// Everything that can be wrong with a config, surfaced as a real error so a
/// CLI (or future HTTP handler) can report it instead of panicking.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{field} must be between 0.0 and 1.0, got {value}")]
    ProbabilityOutOfRange { field: String, value: f64 },

    #[error("{field} must have at least one age band")]
    EmptyBands { field: String },

    #[error(
        "{field} age bands must be sorted ascending by max_age and end with a u8::MAX catch-all"
    )]
    MalformedBands { field: String },

    #[error("{field} must be greater than zero")]
    MustBePositive { field: String },
}

impl SimulationConfig {
    /// Check every probability is in `[0, 1]`, every divisor is positive, and
    /// the age bands are well formed. Call this once before handing the config
    /// to the engine.
    pub fn validate(&self) -> Result<(), ConfigError> {
        check_probability("adherent.starting_heterodoxy", self.adherent.starting_heterodoxy)?;
        check_probability("adherent.conversion_base_rate", self.adherent.conversion_base_rate)?;
        check_probability(
            "adherent.heterodoxy_change_base_rate",
            self.adherent.heterodoxy_change_base_rate,
        )?;

        check_bands("adherent.mortality", &self.adherent.mortality)?;
        check_bands("adherent.birth", &self.adherent.birth)?;

        check_probability(
            "religion.high_heterodoxy_threshold",
            self.religion.high_heterodoxy_threshold,
        )?;
        check_probability("religion.schism_base_rate", self.religion.schism_base_rate)?;

        if self.religion.population_factor_pivot <= 0.0 {
            return Err(ConfigError::MustBePositive {
                field: "religion.population_factor_pivot".to_owned(),
            });
        }

        Ok(())
    }
}

fn check_probability(field: &str, value: f64) -> Result<(), ConfigError> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ConfigError::ProbabilityOutOfRange {
            field: field.to_owned(),
            value,
        })
    }
}

fn check_bands(field: &str, bands: &[AgeBand]) -> Result<(), ConfigError> {
    if bands.is_empty() {
        return Err(ConfigError::EmptyBands {
            field: field.to_owned(),
        });
    }

    for band in bands {
        check_probability(&format!("{field} band rate"), band.rate)?;
    }

    let is_sorted_ascending = bands
        .windows(2)
        .all(|adjacent_bands| adjacent_bands[0].max_age < adjacent_bands[1].max_age);
    let ends_in_catch_all = bands.last().map(|band| band.max_age) == Some(u8::MAX);

    if !is_sorted_ascending || !ends_in_catch_all {
        return Err(ConfigError::MalformedBands {
            field: field.to_owned(),
        });
    }

    Ok(())
}

// The baked-in tuned values live in these per-section `Default` impls — the
// control board you edit to tune between runs without a config file or UI.
// `SimulationConfig` derives `Default` by composing them, and `#[serde(default)]`
// means a `--config` file can override any subset and the rest falls back here.

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            environment: Environment::default(),
            num_generations: 100,
            starting_population: 10_000,
            seed: 67,
        }
    }
}

impl Default for AdherentConfig {
    fn default() -> Self {
        Self {
            starting_heterodoxy: 0.05,
            conversion_base_rate: 0.02,
            heterodoxy_change_base_rate: 0.01,
            mortality: vec![
                AgeBand { max_age: 49, rate: 0.001 },
                AgeBand { max_age: 69, rate: 0.01 },
                AgeBand { max_age: 79, rate: 0.05 },
                AgeBand { max_age: u8::MAX, rate: 0.15 },
            ],
            birth: vec![
                AgeBand { max_age: 12, rate: 0.0 },
                AgeBand { max_age: 17, rate: 0.02 },
                AgeBand { max_age: 25, rate: 0.12 },
                AgeBand { max_age: 35, rate: 0.16 },
                AgeBand { max_age: 45, rate: 0.06 },
                AgeBand { max_age: u8::MAX, rate: 0.0 },
            ],
        }
    }
}

impl Default for ReligionConfig {
    fn default() -> Self {
        Self {
            min_congregation: 50,
            high_heterodoxy_threshold: 0.7,
            population_factor_pivot: 1000.0,
            schism_base_rate: 0.01,
        }
    }
}
