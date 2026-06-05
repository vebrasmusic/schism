use std::collections::{HashMap, HashSet};

use rand::{SeedableRng, rngs::SmallRng};
use slotmap::SlotMap;

use crate::{
    environment::Environment,
    probability::UnitInterval,
    religion::{Adherent, AdherentKey, Religion, ReligionKey},
};

pub struct SimulationConfig {
    environment: Environment,
    num_generations: u32,
}

pub struct Simulation {
    religions: SlotMap<ReligionKey, Religion>,
    adherents: SlotMap<AdherentKey, Adherent>,
    config: SimulationConfig,
    rng: SmallRng,
}

impl Simulation {
    pub fn new(env: Environment, num_generations: u32) -> Self {
        let root_religion = Religion::new(None);

        let mut sim = Self {
            religions: SlotMap::with_key(),
            adherents: SlotMap::with_key(),
            config: SimulationConfig {
                environment: env,
                num_generations,
            },
            rng: SmallRng::seed_from_u64(67),
        };

        let root_religion_id = sim.religions.insert(root_religion);
        let root_adherents: Vec<Adherent> = (0..10000)
            .map(|_| Adherent::new(root_religion_id))
            .collect();

        for adherent in root_adherents {
            sim.adherents.insert(adherent);
        }

        sim
    }

    pub fn run(&mut self) {
        // start loop
        for generation in 0..self.config.num_generations {
            println!("on generation {generation}");
            self.tick();
        }

        println!("simulation ended.")
    }

    fn tick(&mut self) {
        // snapshot which religions exist before this tick, so the readout can
        // flag any that get born this generation. read-only, doesn't affect sim.
        let religions_at_start: HashSet<ReligionKey> = self.religions.keys().collect();

        let mut births: Vec<Adherent> = Vec::new();

        // make map of religion id > vec adherents
        let mut religion_adherents: HashMap<ReligionKey, Vec<AdherentKey>> = HashMap::new();

        for (adherent_id, adherent) in &mut self.adherents {
            // check if they were alrady dead, then don't update
            if adherent.is_dead() {
                continue;
            }

            // do update on adherents
            adherent.update(&mut self.rng);

            // exclude perosn that just died
            if adherent.is_dead() {
                continue;
            }

            // see if they birthed someone
            if adherent.gave_birth(&mut self.rng) {
                births.push(Adherent::new(adherent.religion));
            }

            religion_adherents
                .entry(adherent.religion)
                .or_default()
                .push(adherent_id)
        }

        for child in births {
            self.adherents.insert(child);
        }

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
                religion.mark_extinct();
                continue;
            }

            let adherents: Vec<&Adherent> = adherent_keys
                .iter()
                .map(|adherent_id| &self.adherents[*adherent_id])
                .collect();

            if religion.should_schism(&adherents, &mut self.rng) {
                schisms.push((religion_id, adherent_keys));
            }
        }

        // make new religion, move adherents
        for (parent_id, adherents) in schisms {
            let parent = self.religions.get(parent_id).unwrap();

            let new_sect = Religion::new(Some((parent, parent_id)));

            let new_sect_id = self.religions.insert(new_sect);

            for adherent_id in adherents {
                let adherent = self.adherents.get_mut(*adherent_id).unwrap();

                let converted = adherent.try_conversion(new_sect_id, &mut self.rng);
                if converted {
                    println!("someone converted")
                }
            }
        }

        // self.print_generation_readout(&religions_at_start);
    }

    /// detailed, json-ish dump of the world state at the end of a generation.
    /// purely a readout — touches no simulation state.
    fn print_generation_readout(&self, religions_at_start: &HashSet<ReligionKey>) {
        // tally living adherents per religion, recomputed fresh so it reflects
        // the post-schism / post-conversion state of this generation.
        let mut adherent_counts: HashMap<ReligionKey, usize> = HashMap::new();
        let mut total_people = 0usize;
        for adherent in self.adherents.values() {
            if adherent.is_dead() {
                continue;
            }
            total_people += 1;
            *adherent_counts.entry(adherent.religion).or_default() += 1;
        }

        // one row per religion, biggest congregation first so it scans top-down.
        let mut religion_rows: Vec<(&Religion, usize, bool)> = self
            .religions
            .iter()
            .map(|(religion_id, religion)| {
                let living_adherents = adherent_counts.get(&religion_id).copied().unwrap_or(0);
                let is_new_this_generation = !religions_at_start.contains(&religion_id);
                (religion, living_adherents, is_new_this_generation)
            })
            .collect();
        religion_rows.sort_by(|left, right| right.1.cmp(&left.1));

        let total_religions = religion_rows.len();
        let active_religions = religion_rows
            .iter()
            .filter(|(religion, _, _)| !religion.is_extinct())
            .count();
        let extinct_religions = total_religions - active_religions;
        let new_religions = religion_rows
            .iter()
            .filter(|(_, _, is_new)| *is_new)
            .count();

        println!("{{");
        println!("  \"totals\": {{");
        println!("    \"people\": {total_people},");
        println!("    \"religions\": {total_religions},");
        println!("    \"active\": {active_religions},");
        println!("    \"extinct\": {extinct_religions},");
        println!("    \"new_this_generation\": {new_religions}");
        println!("  }},");
        println!("  \"religions\": [");

        for (index, (religion, living_adherents, is_new)) in religion_rows.iter().enumerate() {
            let parent_name = religion
                .parent
                .and_then(|parent_id| self.religions.get(parent_id))
                .map(|parent| parent.name.as_str())
                .unwrap_or("none");

            let status = if religion.is_extinct() {
                "extinct"
            } else {
                "active"
            };

            let trailing_comma = if index + 1 < religion_rows.len() {
                ","
            } else {
                ""
            };
            let new_marker = if *is_new { "   <-- NEW" } else { "" };

            println!(
                "    {{ \"name\": \"{}\", \"adherents\": {}, \"status\": \"{}\", \"age\": {}, \"parent\": \"{}\", \"new\": {} }}{}{}",
                religion.name,
                living_adherents,
                status,
                religion.age,
                parent_name,
                is_new,
                trailing_comma,
                new_marker,
            );
        }

        println!("  ]");
        println!("}}");
    }
}
