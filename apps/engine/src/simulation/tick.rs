use std::collections::HashSet;

use anyhow::Result;
use slotmap::SlotMap;

use crate::adherent::{Adherent, AdherentKey, AdherentStatus};
use crate::religion::{Religion, ReligionKey};
use crate::simulation::SimulationPhase;

use super::Simulation;
use super::readout::GenerationReadout;

impl Simulation {
    /// advance the world one generation and return the snapshot describing the
    /// resulting state. the readout is built but not serialized here — the run
    /// loop keeps the final generation's and emits it once at the end.
    pub(super) fn tick(&mut self) -> Result<GenerationReadout> {
        if let SimulationPhase::Founding = self.phase
            && self.adherents.len() > 100_000
        {
            self.phase = SimulationPhase::Expansion
        }

        // snapshot which religions exist before this tick, so the readout can
        // flag any that get born this generation. read-only, doesn't affect sim.
        let religions_at_start: HashSet<ReligionKey> = self.religions.keys().collect();

        // advance the world clock one generation. religions born this tick are
        // stamped with this year, and any that die are stamped extinct with it.
        self.current_year += self.config.adherent.generation_length_yrs as u32;
        let current_year = self.current_year;

        // get rid of any adherents that died
        self.adherents
            .retain(|_, adherent| adherent.status == AdherentStatus::Alive);

        let mean_heterodoxy = self.mean_living_heterodoxy();

        let religion_adherents = self.advance_adherents(mean_heterodoxy)?;

        let mut schisms = Vec::new();

        for (religion_id, religion) in &mut self.religions {
            if religion.is_extinct() {
                continue;
            }

            let adherent_keys = religion_adherents
                .get(&religion_id)
                .map(|keys| keys.as_slice())
                .unwrap_or(&[]);

            if adherent_keys.is_empty() {
                religion.mark_extinct(current_year);
                continue;
            }

            let adherents: Vec<&Adherent> = adherent_keys
                .iter()
                .map(|adherent_id| &self.adherents[*adherent_id])
                .collect();

            if religion.should_schism(
                &adherents,
                mean_heterodoxy,
                &self.config.religion,
                &mut self.rng,
            ) {
                schisms.push((religion_id, adherent_keys));
            }
        }

        // make new religion, move adherents
        for (parent_id, adherents) in schisms {
            let parent = self.religions.get(parent_id).unwrap();

            let new_sect = Religion::new(Some((parent, parent_id)), current_year, &mut self.rng)?;

            let new_sect_id = self.religions.insert(new_sect);

            for adherent_id in adherents {
                let adherent = self.adherents.get_mut(*adherent_id).unwrap();

                let converted = adherent.try_conversion(
                    new_sect_id,
                    self.config.religion.high_heterodoxy_threshold,
                    &self.config.adherent,
                    &mut self.rng,
                );
                // if converted {
                //     println!("someone converted")
                // }
            }
        }

        Ok(self.build_generation_readout(&religions_at_start, mean_heterodoxy))
    }
}
