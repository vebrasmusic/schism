use core::panic;
use std::collections::HashMap;

use ahash::RandomState;
use anyhow::{Context, Result};
use rand_distr::num_traits::ToPrimitive;
use rand_distr::{Binomial, Distribution};

use crate::adherent::{Adherent, AdherentKey, AdherentStatus};
use crate::histogram::{AgeBand, Count, HeterodoxyBin, PopulationHistogram};
use crate::probability::{
    UnitInterval, bin_adherents, create_child_heterodoxy_distribution, flip_weighted_coin,
};
use crate::religion::Religion;

use super::Simulation;

impl Simulation {
    /// per religion, calculate how many of each bin die and decrement.
    /// `total_population` is the world's living count at the start of the tick;
    /// mortality is scaled by how crowded the environment is relative to its
    /// carrying capacity, so growth bends into an S-curve instead of exploding.
    pub(super) fn remove_dead(&mut self, total_population: u64) -> Result<()> {
        // density-dependent multiplier on mortality: 1.0 when the population
        // exactly fills the environment's carrying capacity, below 1.0 while
        // there's room to grow, and above 1.0 once it's overcrowded.
        let crowding_factor =
            total_population as f64 / self.config.environment.carrying_capacity as f64;

        for (_, religion) in &mut self.religions {
            match religion {
                Religion::Active { adherents, .. } => {
                    for (age_band, age_band_vector) in adherents.iter_bands_mut() {
                        let base_mortality = self
                            .config
                            .adherent
                            .mortality_rate(age_band.get_age(
                                self.config.adherent.num_age_bins,
                                self.config.adherent.max_age_yrs,
                            ) as u8)
                            .value();

                        // clamp: crowding can push the scaled rate past 1.0, but a
                        // probability can't exceed 1 (everyone in the bin dies).
                        let mortality = (base_mortality * crowding_factor).clamp(0.0, 1.0);

                        for (_, count) in age_band_vector {
                            let num_dead =
                                Binomial::new(count.value(), mortality)?.sample(&mut self.rng);

                            // make sure i subtract the num dead here
                            count.adjust(-(num_dead as i64)).with_context(|| {
                                format!(
                                    "remove_dead: subtracting {num_dead} dead from count={}",
                                    count.value()
                                )
                            })?;
                        }
                    }
                }
                Religion::Extinct { .. } => {}
            }
        }

        Ok(())
    }

    /// age everyone up by one generation. a tick is `generation_length_yrs` long,
    /// so people advance `generation_length_yrs / years_per_band` age bands, where
    /// `years_per_band = max_age / num_age_bins`. we prepend that many empty
    /// youngest bands (band 0 is refilled by the birth step) to shift everyone up,
    /// then truncate back to the original band count so whoever aged past the top
    /// is dropped.
    pub(super) fn increment_age(&mut self) -> Result<()> {
        let years_per_band =
            self.config.adherent.max_age_yrs as usize / self.config.adherent.num_age_bins;
        let bands_to_advance =
            self.config.adherent.generation_length_yrs as usize / years_per_band;
        let heterodoxy_row_width = self.config.adherent.num_heterodoxy_bins + 1;

        for (_, religion) in &mut self.religions {
            match religion {
                Religion::Active { adherents, .. } => {
                    let old_counts = adherents.take_counts();
                    let original_length = old_counts.len();

                    // shift everyone up by prepending `bands_to_advance` empty youngest
                    // bands, then push all the existing bands on after them...
                    let mut new_counts =
                        vec![vec![Count(0); heterodoxy_row_width]; bands_to_advance];
                    new_counts.extend(old_counts);

                    // ...then drop the bands that aged past the top, restoring the
                    // original band count.
                    new_counts.truncate(original_length);

                    adherents.swap_counts(new_counts);
                }
                Religion::Extinct { .. } => {}
            }
        }

        Ok(())
    }

    pub(super) fn add_births(&mut self) -> Result<()> {
        for (_, religion) in &mut self.religions {
            match religion {
                Religion::Active { adherents, .. } => {
                    let mut birth_counts_per_heterodoxy_bin: Vec<u64> =
                        vec![0; self.config.adherent.num_heterodoxy_bins + 1];

                    for (age_band, age_band_vector) in adherents.iter_bands() {
                        let birth_rate = self
                            .config
                            .adherent
                            .birth_rate(age_band.get_age(
                                self.config.adherent.num_age_bins,
                                self.config.adherent.max_age_yrs,
                            ) as u8)
                            .value();

                        for (heterodoxy_bin, count) in age_band_vector {
                            let num_born =
                                Binomial::new(count.value(), birth_rate)?.sample(&mut self.rng);

                            birth_counts_per_heterodoxy_bin[heterodoxy_bin.value()] += num_born;
                        }
                    }

                    for (bin, count) in birth_counts_per_heterodoxy_bin.iter().enumerate() {
                        if *count == 0 {
                            continue;
                        }
                        adherents
                            .adjust(AgeBand::from(0), HeterodoxyBin::from(bin), *count as i64)
                            .with_context(|| {
                                format!("add_births: adjusting het_bin={bin} by +{count}")
                            })?;
                    }
                }
                Religion::Extinct { .. } => {}
            }
        }

        Ok(())
    }
}
