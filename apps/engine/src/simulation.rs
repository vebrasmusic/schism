use rand::{SeedableRng, rngs::SmallRng};
use slotmap::SlotMap;

use crate::{
    adherent::{Adherent, AdherentKey},
    environment::Environment,
    religion::{Religion, ReligionKey},
};

mod adherent;
mod readout;
mod tick;

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
}
