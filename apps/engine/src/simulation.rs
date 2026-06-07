use rand::{SeedableRng, rngs::SmallRng};
use slotmap::SlotMap;

use crate::{
    adherent::{Adherent, AdherentKey},
    config::SimulationConfig,
    religion::{Religion, ReligionKey},
};

mod adherent;
mod readout;
mod tick;

pub struct Simulation {
    religions: SlotMap<ReligionKey, Religion>,
    adherents: SlotMap<AdherentKey, Adherent>,
    config: SimulationConfig,
    rng: SmallRng,
}

impl Simulation {
    pub fn new(config: SimulationConfig) -> Self {
        let root_religion = Religion::new(None);

        let mut sim = Self {
            religions: SlotMap::with_key(),
            adherents: SlotMap::with_key(),
            rng: SmallRng::seed_from_u64(config.world.seed),
            config,
        };

        let root_religion_id = sim.religions.insert(root_religion);
        let root_adherents: Vec<Adherent> = (0..sim.config.world.starting_population)
            .map(|_| Adherent::new(root_religion_id, &sim.config.adherent))
            .collect();

        for adherent in root_adherents {
            sim.adherents.insert(adherent);
        }

        sim
    }

    pub fn run(&mut self) {
        // start loop
        for generation in 0..self.config.world.num_generations {
            println!("on generation {generation}");
            self.tick();
        }

        println!("simulation ended.")
    }
}
