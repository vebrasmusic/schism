use anyhow::{Context, Result};
use rand_distr::{Binomial, Distribution};

use crate::{
    adherent,
    histogram::{AgeBand, HeterodoxyBin, PopulationHistogram},
    religion::Religion,
    simulation::Simulation,
};

impl Simulation {
    /// population-weighted mean heterodoxy across every living adherent in the simulation.
    /// returns 0.0 for an empty world.
    pub fn mean_heterodoxy(&self) -> f64 {
        let mut weighted_sum = 0.0f64;
        let mut total = 0u64;

        for religion in self.religions.values() {
            let pop = religion.total_population();
            if pop > 0 {
                weighted_sum += religion.mean_heterodoxy() * pop as f64;
                total += pop;
            }
        }

        if total == 0 {
            0.0
        } else {
            weighted_sum / total as f64
        }
    }

    /// total all pop in all religions, check if they're extinct after the whole birth /death cycle
    pub(super) fn mark_extinct_religions(&mut self, current_date: u32) {
        for (_, religion) in &mut self.religions {
            if religion.total_population() == 0 {
                religion.mark_extinct(current_date);
            }
        }
    }

    pub(super) fn schism_religions(&mut self) -> Result<()> {
        // for each religion, see if it triggers
        let mut new_religions: Vec<Religion> = vec![];
        for (religion_key, religion) in &mut self.religions {
            match religion {
                Religion::Active {
                    adherents,
                    name: religion_name,
                    ..
                } => {
                    let threshold_as_bin = HeterodoxyBin::from_heterodoxy(
                        self.config.religion.high_heterodoxy_threshold.value(),
                        self.config.adherent.num_heterodoxy_bins,
                    )
                    .value();

                    let mut count_of_heterodox = 0;

                    for (_, age_band_vector) in adherents.iter_bands() {
                        let count_for_band: u64 = age_band_vector
                            .into_iter()
                            .skip(threshold_as_bin)
                            .map(|(_, count)| count.value())
                            .sum();

                        count_of_heterodox += count_for_band;
                    }

                    let frac = count_of_heterodox as f64 / adherents.total() as f64;

                    // // check if we're passing percentage threshold, if not no schism
                    if frac < self.config.religion.high_heterodoxy_max_fraction.value() {
                        continue;
                    }

                    // past this, we have schismed

                    let mut new_religion = Religion::new(
                        Some((religion_name, religion_key)),
                        self.current_year,
                        &mut self.rng,
                        PopulationHistogram::new(
                            self.config.adherent.num_heterodoxy_bins,
                            self.config.adherent.num_age_bins,
                        ),
                    )
                    .context("schisming religion")?;

                    let Religion::Active {
                        adherents: new_religion_adherents,
                        ..
                    } = &mut new_religion
                    else {
                        panic!("new religion shouldn't be extinct");
                    };

                    // move over adhernets to this new religion
                    for (age_band, age_band_vec) in adherents.iter_mut().enumerate() {
                        for (heterodoxy_bin, count) in
                            age_band_vec.iter_mut().enumerate().skip(threshold_as_bin)
                        {
                            let num_converts = Binomial::new(
                                count.value(),
                                heterodoxy_bin as f64
                                    / self.config.adherent.num_heterodoxy_bins as f64
                                    * self.config.adherent.conversion_base_rate.value(),
                            )?
                            .sample(&mut self.rng);

                            // get rid of it from the current religion
                            count.adjust(-(num_converts as i64))?;

                            // add to new religion
                            new_religion_adherents.adjust(
                                AgeBand::from(age_band),
                                HeterodoxyBin::from(heterodoxy_bin),
                                num_converts as i64,
                            )?;
                        }
                    }

                    new_religions.push(new_religion);
                }
                Religion::Extinct { .. } => {}
            }
        }

        new_religions.into_iter().for_each(|r| {
            self.religions.insert(r);
        });

        Ok(())
    }
}
