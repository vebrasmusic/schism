use std::collections::HashSet;

use anyhow::Result;
use slotmap::SlotMap;

use crate::adherent::{Adherent, AdherentKey, AdherentStatus};
use crate::religion::{Religion, ReligionKey};

use super::Simulation;

impl Simulation {
    pub(super) fn tick(&mut self) -> Result<()> {
        // snapshot which religions exist before this tick, so the readout can
        // flag any that get born this generation. read-only, doesn't affect sim.
        let religions_at_start: HashSet<ReligionKey> = self.religions.keys().collect();

        // get rid of any adherents that died
        self.adherents
            .retain(|_, adherent| adherent.status == AdherentStatus::Alive);

        let mean_heterodoxy = self
            .adherents
            .iter()
            .map(|(_, adherent)| adherent.heterodoxy.value())
            .sum::<f64>()
            / self.adherents.len() as f64;

        let religion_adherents = self.advance_adherents(mean_heterodoxy)?;

        let mut schisms = Vec::new();

        for (religion_id, religion) in &mut self.religions {
            if religion.is_extinct() {
                continue;
            }

            religion.age += self.config.adherent.generation_length_yrs as u32;

            let adherent_keys = religion_adherents
                .get(&religion_id)
                .map(|keys| keys.as_slice())
                .unwrap_or(&[]);

            if adherent_keys.is_empty() {
                religion.mark_extinct();
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

            let new_sect = Religion::new(Some((parent, parent_id)), &mut self.rng)?;

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

        self.print_generation_readout(&religions_at_start, mean_heterodoxy);
        Ok(())
    }
}
