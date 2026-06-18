use std::collections::HashMap;

use anyhow::{Context, Result};
use rand_distr::Distribution;
use rand_distr::num_traits::ToPrimitive;

use crate::adherent::{Adherent, AdherentKey, AdherentStatus};
use crate::probability::{
    UnitInterval, bin_adherents, create_child_heterodoxy_distribution, flip_weighted_coin,
};
use crate::religion::ReligionKey;

use super::Simulation;

impl Simulation {
    /// age + update every living adherent, birth any children, and group the
    /// survivors by religion. returns the religion -> living adherents map the
    /// schism pass consumes.
    pub(super) fn advance_adherents(
        &mut self,
        population_mean_heterodoxy: f64,
    ) -> Result<HashMap<ReligionKey, Vec<AdherentKey>>> {
        let mut births: Vec<Adherent> = Vec::new();

        // make map of religion id > vec adherents
        let mut religion_adherents: HashMap<ReligionKey, Vec<AdherentKey>> = HashMap::new();

        for (adherent_id, adherent) in &mut self.adherents {
            // check if they were alrady dead, then don't update
            if adherent.is_dead() {
                continue;
            }

            // do update on adherents
            adherent.update(&self.config.adherent, &mut self.rng);

            // exclude perosn that just died
            if adherent.is_dead() {
                continue;
            }

            religion_adherents
                .entry(adherent.religion)
                .or_default()
                .push(adherent_id)
        }

        let adherents: Vec<&Adherent> = self
            .adherents
            .iter()
            .filter(|(_, a)| a.status == AdherentStatus::Alive)
            .map(|(_, a)| a)
            .collect();

        let bins = bin_adherents(adherents, self.config.world.cohort_heterodoxy_bins);

        // for each bin, figure out len and average age > average birth rate
        for (bin, adherents) in bins {
            let num_adherents_in_bin = adherents.len();

            let mut religion_totals_map: HashMap<ReligionKey, u64> = HashMap::new();

            // TODO: try doing a weighted average instead at some poimt
            for adherent in &adherents {
                religion_totals_map
                    .entry(adherent.religion)
                    .and_modify(|v| *v += 1)
                    .or_insert(1);
            }

            // println!("religion_totals_map: {:?}", religion_totals_map);

            // for loop thru each adherent in bin, gather if they birthed a kid
            // by filtering for how many came back true
            let num_children_born: usize = adherents
                .iter()
                .filter(|a| {
                    flip_weighted_coin(self.config.adherent.birth_rate(a.age), &mut self.rng)
                })
                .count();

            // if no children born, we can just skip the rest here
            if num_children_born == 0 {
                continue;
            }

            // create distr. for heterodoxy vals
            let bin_mean_heterodoxy = bin as f64 / self.config.world.cohort_heterodoxy_bins as f64;

            let distr = create_child_heterodoxy_distribution(
                bin_mean_heterodoxy,
                population_mean_heterodoxy,
                &self.config.adherent,
            )
            .context("error creating beta distribution for child heterodoxy")?;

            // TODO: if slow, can ignore per bin religion splits and just do one population wide calc.
            for (religion, num_adherents) in religion_totals_map {
                let percentage = num_adherents as f64 / num_adherents_in_bin as f64;

                if percentage > 1.0 {
                    panic!("percentage greater than 1");
                }

                // println!("percentage {}", percentage);
                let num_births_in_religion = (percentage * num_children_born as f64)
                    .ceil()
                    .to_u64()
                    .unwrap();

                // println!(
                //     "num births in religion {}: {}",
                //     religion, num_births_in_religion
                // );

                for _ in 0..num_births_in_religion {
                    births.push(Adherent::new(
                        religion,
                        UnitInterval::new(distr.sample(&mut self.rng)),
                        Some(0),
                    ));
                }
            }
        }

        for child in births {
            self.adherents.insert(child);
        }

        Ok(religion_adherents)
    }
}
