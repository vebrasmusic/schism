use anyhow::Result;
use rand::{SeedableRng, rngs::SmallRng};
use rand_distr::Distribution;
use slotmap::SlotMap;

use crate::{
    adherent::{Adherent, AdherentKey},
    config::SimulationConfig,
    probability::{
        UnitInterval, create_initial_population_age_distribution,
        create_initial_population_heterodoxy_distribution,
    },
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
    pub fn new(config: SimulationConfig) -> Result<Self> {
        let root_religion = Religion::new(None);

        let mut sim = Self {
            religions: SlotMap::with_key(),
            adherents: SlotMap::with_key(),
            rng: SmallRng::seed_from_u64(config.world.seed),
            config,
        };

        let root_religion_id = sim.religions.insert(root_religion);

        let population_heterodoxy_distr =
            create_initial_population_heterodoxy_distribution(&sim.config.adherent)?;
        let population_age_distr =
            create_initial_population_age_distribution(&sim.config.adherent)?;

        // oldest age someone can START at — one below the hard cap, so nobody
        // begins already past it (and thus dead on the first tick).
        let oldest_starting_age = sim.config.adherent.max_age_yrs.saturating_sub(1);

        for _ in 0..sim.config.world.starting_population {
            // draw heterodoxy AND age per adherent so the starting population is
            // actually spread out, not every member a clone of one sample.
            let heterodoxy = UnitInterval::new(population_heterodoxy_distr.sample(&mut sim.rng));

            let sampled_age = population_age_distr.sample(&mut sim.rng);
            let age = sampled_age.round().clamp(0.0, oldest_starting_age as f64) as u8;

            sim.adherents
                .insert(Adherent::new(root_religion_id, heterodoxy, Some(age)));
        }

        Ok(sim)
    }

    pub fn run(&mut self) -> Result<()> {
        // start loop
        for generation in 0..self.config.world.num_generations {
            println!("on generation {generation}");
            self.tick()?;
        }

        println!("simulation ended.");
        Ok(())
    }
}
