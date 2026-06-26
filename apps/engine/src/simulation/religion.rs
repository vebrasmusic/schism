use anyhow::{Context, Result};
use rand_distr::{Binomial, Distribution};

use crate::{
    adherent,
    histogram::{AgeBand, HeterodoxyBin, PopulationHistogram},
    religion::{Religion, ReligionKey},
    simulation::Simulation,
};

impl Simulation {
    /// population-weighted mean heterodoxy across every living adherent in the simulation.
    /// returns 0.0 for an empty world.
    pub fn mean_heterodoxy(&self) -> f64 {
        let mut weighted_sum = 0.0f64;
        let mut total = 0u64;

        for religion in self.active_religions.values() {
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
        let extinct_keys: Vec<ReligionKey> = self
            .active_religions
            .iter()
            .filter(|(_, r)| r.total_population() == 0)
            .map(|(k, _)| k)
            .collect();

        for key in extinct_keys {
            if let Some(mut religion) = self.active_religions.remove(key) {
                religion.mark_extinct(current_date);
                self.extinct_religions.push((key, religion));
            }
        }
    }

    pub(super) fn schism_religions(&mut self) -> Result<()> {
        // for each religion, see if it triggers
        let mut new_religions: Vec<Religion> = vec![];
        for (religion_key, religion) in &mut self.active_religions {
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

                    // pass 1: pull the converts out of the parent, remembering
                    // where each came from, and accumulate their mean heterodoxy.
                    // heterodoxy isn't age-specific, so it's one mean pooled across
                    // every age band.
                    let mut converts: Vec<(usize, usize, u64)> = vec![];
                    let mut convert_weighted_sum = 0.0f64;
                    let mut convert_total = 0.0f64;

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

                            if num_converts == 0 {
                                continue;
                            }

                            // remove the converts from the parent religion
                            count.adjust(-(num_converts as i64))?;

                            let heterodoxy_value = heterodoxy_bin as f64
                                / self.config.adherent.num_heterodoxy_bins as f64;
                            convert_weighted_sum += heterodoxy_value * num_converts as f64;
                            convert_total += num_converts as f64;
                            converts.push((age_band, heterodoxy_bin, num_converts));
                        }
                    }

                    // nobody actually broke away this round — don't spawn a
                    // stillborn religion.
                    if convert_total == 0.0 {
                        continue;
                    }

                    let mean_convert_heterodoxy = convert_weighted_sum / convert_total;

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

                    // pass 2: re-center converts around their own mean. the
                    // daughter's orthodoxy is its members' center of mass: a convert
                    // who sat at that mean becomes the new mainstream (heterodoxy 0),
                    // those above stay heterodox and seed the daughter's own future
                    // schisms, and those below the mean collapse to orthodox. rescale
                    // [mean, 1.0] onto [0, 1]:
                    //   new_het = max(0, old_het - mean) / (1 - mean)
                    let heterodoxy_span = (1.0 - mean_convert_heterodoxy).max(f64::EPSILON);

                    for (age_band, heterodoxy_bin, num_converts) in converts {
                        let old_heterodoxy =
                            heterodoxy_bin as f64 / self.config.adherent.num_heterodoxy_bins as f64;
                        let recentered_heterodoxy =
                            ((old_heterodoxy - mean_convert_heterodoxy) / heterodoxy_span).max(0.0);
                        let recentered_bin = HeterodoxyBin::from_heterodoxy(
                            recentered_heterodoxy,
                            self.config.adherent.num_heterodoxy_bins,
                        );

                        new_religion_adherents.adjust(
                            AgeBand::from(age_band),
                            recentered_bin,
                            num_converts as i64,
                        )?;
                    }

                    new_religions.push(new_religion);
                }
                Religion::Extinct { .. } => {}
            }
        }

        new_religions.into_iter().for_each(|r| {
            self.active_religions.insert(r);
        });

        Ok(())
    }
}
