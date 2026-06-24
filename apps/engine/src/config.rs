use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::environment::Environment;
use crate::probability::{PositiveReal, UnitInterval};

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

    /// living-adherent count at which the sim switches from Individual to Cohort
    /// scale; beyond this the population is too large to track one-by-one
    pub cohort_scale_threshold: usize,

    /// number of heterodoxy buckets used when collapsing adherents into cohorts
    pub cohort_heterodoxy_bins: usize,
}

/// Per-adherent lifecycle rates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdherentConfig {
    /// length in yrs of a generation (for aging up adherents)
    pub generation_length_yrs: u8,

    /// hard upper bound on age. the mortality bands model the actuarial table
    /// below this; at or beyond it survival is treated as impossible and the
    /// adherent is marked dead. keeps the 20-yr age jumps from walking past the
    /// table (where mortality reads as ~0) and overflowing `age` (a u8).
    pub max_age_yrs: u8,

    /// 0.2 => mostly orthodox society, 0.5 => balanced, 0.8 => mostly heterodox
    pub population_mean_heterodoxy: UnitInterval,

    /// controls diversity of population, conc. around the mean
    /// 5: very diverse
    /// 20: moderate
    /// 100: homogenous
    pub population_heterodoxy_concentration: PositiveReal,

    /// mean starting age (yrs) for the initial population's age distribution, so
    /// the sim doesn't begin with everyone the same age
    pub population_mean_age_yrs: PositiveReal,

    /// spread (std dev, yrs) of the initial population's age distribution
    pub population_age_spread_yrs: PositiveReal,

    /// heritability (parental / societal influence) over child's orthodoxy
    pub parental_heterodoxy_influence: UnitInterval,

    /// conc. of the children around caclcualted expected value
    pub child_heterodoxy_concentration: PositiveReal,

    /// base chance to convert when a new sect appears, scaled by heterodoxy
    pub conversion_base_rate: UnitInterval,

    /// base per-tick heterodoxy drift, scaled by heterodoxy
    pub heterodoxy_change_base_rate: UnitInterval,

    /// number of heterodoxy buckets in the population histogram
    pub num_heterodoxy_bins: usize,

    /// number of age buckets in the population histogram
    pub num_age_bins: usize,

    /// chance of death per tick below `max_age_yrs`, looked up by age band
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

    /// the belief fault line. heterodoxy strictly above this both (1) counts
    /// toward the "high heterodoxy" share that drives schism likelihood and
    /// (2) marks the wing that actually breaks away when a schism fires — the
    /// orthodox majority below it stays with the parent faith.
    pub high_heterodoxy_threshold: UnitInterval,

    /// congregation size at which the population factor saturates to 1.0
    pub population_factor_pivot: f64,

    /// base multiplier on the per-tick schism chance
    pub schism_base_rate: UnitInterval,
}

/// A single age bracket: applies `rate` to everyone with `age <= max_age`,
/// taking the first matching band. Birth bands end in a `u8::MAX` catch-all;
/// mortality bands instead stop at the `max_age_yrs` cap (everyone older is
/// force-killed). `validate` enforces the right tail for each.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeBand {
    pub max_age: u8,
    pub rate: UnitInterval,
}

impl AdherentConfig {
    /// mortality rate for an adherent of the given age
    pub fn mortality_rate(&self, age: u8) -> UnitInterval {
        Self::lookup(&self.mortality, age)
    }

    /// birth rate for an adherent of the given age
    pub fn birth_rate(&self, age: u8) -> UnitInterval {
        Self::lookup(&self.birth, age)
    }

    /// rate from the first band covering `age`. callers only look up ages a
    /// matching band exists for — birth bands end in a `u8::MAX` catch-all, and
    /// mortality is only queried below the `max_age_yrs` cap, which `validate`
    /// guarantees the bands reach. the zero fallback only fires if one of those
    /// invariants is somehow broken.
    fn lookup(bands: &[AgeBand], age: u8) -> UnitInterval {
        bands
            .iter()
            .find(|band| age <= band.max_age)
            .map(|band| band.rate)
            .unwrap_or(UnitInterval::new(0.0))
    }
}

/// Everything that can be wrong with a config that the type system doesn't
/// already rule out (every probability is a `UnitInterval`, so range is enforced
/// at deserialize time). Surfaced as a real error so a CLI (or future HTTP
/// handler) can report it instead of panicking.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{field} must have at least one age band")]
    EmptyBands { field: String },

    #[error("{field} age bands must be sorted strictly ascending by max_age")]
    UnsortedBands { field: String },

    #[error("{field} age bands must end with a u8::MAX catch-all band")]
    MissingCatchAll { field: String },

    #[error(
        "adherent.mortality age bands must cover every age below adherent.max_age_yrs ({cap}); \
         the last band only reaches {last_covered}"
    )]
    MortalityCapGap { cap: u8, last_covered: u8 },

    #[error("{field} must be greater than zero")]
    MustBePositive { field: String },
}

impl SimulationConfig {
    /// Check the structural invariants types can't express: divisors are
    /// positive and the age bands are well formed. Probability ranges are
    /// already guaranteed by `UnitInterval`. Call this once before handing the
    /// config to the engine.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // birth has no upper age cap, so its bands must end in a u8::MAX
        // catch-all that covers everyone past the last real bracket.
        check_sorted_nonempty("adherent.birth", &self.adherent.birth)?;
        if self.adherent.birth.last().map(|band| band.max_age) != Some(u8::MAX) {
            return Err(ConfigError::MissingCatchAll {
                field: "adherent.birth".to_owned(),
            });
        }

        // mortality is capped: `Adherent::should_die` force-kills anyone at or
        // past `max_age_yrs`, so the bands don't need a catch-all — but they must
        // reach the cap, or ages just below it would fall through to zero
        // mortality and live forever.
        check_sorted_nonempty("adherent.mortality", &self.adherent.mortality)?;
        let last_covered = self
            .adherent
            .mortality
            .last()
            .map(|band| band.max_age)
            .unwrap_or(0);
        if (last_covered as u16) + 1 < self.adherent.max_age_yrs as u16 {
            return Err(ConfigError::MortalityCapGap {
                cap: self.adherent.max_age_yrs,
                last_covered,
            });
        }

        if self.religion.population_factor_pivot <= 0.0 {
            return Err(ConfigError::MustBePositive {
                field: "religion.population_factor_pivot".to_owned(),
            });
        }

        Ok(())
    }
}

fn check_sorted_nonempty(field: &str, bands: &[AgeBand]) -> Result<(), ConfigError> {
    if bands.is_empty() {
        return Err(ConfigError::EmptyBands {
            field: field.to_owned(),
        });
    }

    let is_sorted_ascending = bands
        .windows(2)
        .all(|adjacent_bands| adjacent_bands[0].max_age < adjacent_bands[1].max_age);

    if !is_sorted_ascending {
        return Err(ConfigError::UnsortedBands {
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
            num_generations: 150,
            starting_population: 500,
            seed: 67,
            cohort_scale_threshold: 1_000_000,
            cohort_heterodoxy_bins: 100,
        }
    }
}

// NOTE (temporary, just so I remember why these values were chosen):
//
// Modeling a fairly realistic human society where most people are somewhat
// conventional, but not rigidly so:
//
// | Parameter                  | Default | Why                                                     |
// |----------------------------|---------|---------------------------------------------------------|
// | population_mean_heterodoxy | 0.25    | Most people are closer to mainstream than fringe.       |
// | population_concentration   | 12–20   | Noticeable diversity without making extremes common.    |
// | parent_influence           | 0.6–0.8 | Parents matter a lot, but society still has influence.  |
// | child_concentration        | 20–40   | Children resemble parents somewhat but aren't copies.   |
//
// If I had to pick one exact set:
//   population_mean_heterodoxy = 0.25
//   population_concentration   = 15
//   parent_influence           = 0.7
//   child_concentration        = 30
//
// Interpretation:
//   * Average person is mildly orthodox.
//   * Some heterodox people exist, but they're not dominant.
//   * Parents explain most of a child's tendency.
//   * Children are usually similar to parents, but you still get occasional
//     rebels and conformists.
//
// A few examples from those defaults:
//
//   | Parent | Expected Child |
//   |--------|----------------|
//   | 0.1    | 0.145          |
//   | 0.3    | 0.285          |
//   | 0.8    | 0.635          |
//   | 1.0    | 0.775          |
//
// Note how very heterodox parents tend to produce heterodox children, but not
// usually maximally heterodox children — that's regression to the mean.
//
// If heterodoxy is supposed to be rare and prestigious (independent thinkers,
// heretics, innovators), push the starting mean lower:
//   population_mean_heterodoxy = 0.15
//   population_concentration   = 20
// That produces a society where genuinely heterodox individuals are uncommon
// but still naturally emerge.
impl Default for AdherentConfig {
    fn default() -> Self {
        Self {
            generation_length_yrs: 20,
            max_age_yrs: 100,
            population_mean_heterodoxy: UnitInterval::new(0.25),
            population_heterodoxy_concentration: PositiveReal::new(15.0),
            population_mean_age_yrs: PositiveReal::new(30.0),
            population_age_spread_yrs: PositiveReal::new(15.0),
            parental_heterodoxy_influence: UnitInterval::new(0.6),
            child_heterodoxy_concentration: PositiveReal::new(30.0),
            num_heterodoxy_bins: 500,
            num_age_bins: 20,
            conversion_base_rate: UnitInterval::new(0.7),
            heterodoxy_change_base_rate: UnitInterval::new(0.01),
            // per-GENERATION (20-yr) probabilities, converted from the old
            // per-year rates via p_gen = 1 - (1 - p_year)^20. beyond the last
            // band, `max_age_yrs` force-kills everyone (see Adherent::should_die).
            mortality: vec![
                AgeBand {
                    max_age: 49,
                    rate: UnitInterval::new(0.02), // 0.001/yr
                },
                AgeBand {
                    max_age: 69,
                    rate: UnitInterval::new(0.18), // 0.01/yr
                },
                AgeBand {
                    max_age: 79,
                    rate: UnitInterval::new(0.64), // 0.05/yr
                },
                AgeBand {
                    max_age: 99,
                    rate: UnitInterval::new(0.96), // 0.15/yr
                },
            ],
            // per-GENERATION (20-yr) chance of producing a child in the tick.
            // the model births at most one child per tick, so with reproductive
            // ticks at age 20 and 40 a lineage averages ~1.1 children — slightly
            // above the asexual replacement level of 1.0, so the population grows
            // gently instead of collapsing.
            birth: vec![
                AgeBand {
                    max_age: 14,
                    rate: UnitInterval::new(0.0),
                },
                AgeBand {
                    max_age: 29,
                    rate: UnitInterval::new(0.7),
                },
                AgeBand {
                    max_age: 44,
                    rate: UnitInterval::new(0.4),
                },
                AgeBand {
                    max_age: 59,
                    rate: UnitInterval::new(0.05),
                },
                AgeBand {
                    max_age: u8::MAX,
                    rate: UnitInterval::new(0.0),
                },
            ],
        }
    }
}

impl Default for ReligionConfig {
    fn default() -> Self {
        Self {
            min_congregation: 10,
            // the fault line a schism splits along. sits well above the
            // population mean (0.25) so the breakaway wing is a genuine
            // heterodox minority, but low enough that the wing is non-empty —
            // at 0.7 essentially nobody clears it and every new sect is
            // stillborn.
            high_heterodoxy_threshold: UnitInterval::new(0.5),
            population_factor_pivot: 1000.0,
            schism_base_rate: UnitInterval::new(0.03),
        }
    }
}
