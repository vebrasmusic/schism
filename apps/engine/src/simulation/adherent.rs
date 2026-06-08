use std::collections::HashMap;

use anyhow::Result;

use crate::adherent::{self, Adherent, AdherentKey};
use crate::religion::ReligionKey;

use super::Simulation;

impl Simulation {
    /// age + update every living adherent, birth any children, and group the
    /// survivors by religion. returns the religion -> living adherents map the
    /// schism pass consumes.
    pub(super) fn advance_adherents(
        &mut self,
        mean_heterodoxy: f64,
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

            // see if they birthed someone
            let child =
                adherent.try_birth(mean_heterodoxy, &self.config.adherent, &mut self.rng)?;

            match child {
                Some(child) => births.push(child),
                _ => (),
            }

            religion_adherents
                .entry(adherent.religion)
                .or_default()
                .push(adherent_id)
        }

        for child in births {
            self.adherents.insert(child);
        }

        Ok(religion_adherents)
    }
}
