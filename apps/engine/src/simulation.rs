use anyhow::{Context, Result};
use rand::{SeedableRng, rngs::SmallRng};
use rand_distr::Distribution;
use slotmap::SlotMap;

use crate::{
    adherent::{Adherent, AdherentKey},
    config::SimulationConfig,
    histogram::PopulationHistogram,
    probability::{
        UnitInterval, create_initial_population_age_distribution,
        create_initial_population_heterodoxy_distribution,
    },
    religion::{Religion, ReligionKey},
};

mod adherent;
mod probability;
mod readout;
mod religion;
mod tick;

pub enum SimulationScale {
    Individual,
    Cohort,
    Aggregate,
}

pub struct Simulation {
    active_religions: SlotMap<ReligionKey, Religion>,
    extinct_religions: Vec<(ReligionKey, Religion)>,
    config: SimulationConfig,
    scale: SimulationScale,
    rng: SmallRng,
    /// world clock in years, counting up from 0; advanced one generation per tick
    current_year: u32,
}

impl Simulation {
    pub fn new(mut config: SimulationConfig) -> Result<Self> {
        // resolve the chosen environment's tunables (carrying capacity, ...) and
        // fold them into the config so the rest of the engine reads them off
        // `self.config.environment`, same as the other sub-configs.
        config.environment = config.world.environment.config();

        let mut sim = Self {
            active_religions: SlotMap::with_key(),
            extinct_religions: Vec::new(),
            rng: SmallRng::seed_from_u64(config.world.seed),
            scale: SimulationScale::Individual,
            config,
            current_year: 0,
        };

        // set this up as a separate func
        // ok to do this one by one bc we start w/ a small pop.
        let mut root_adherents = PopulationHistogram::new(
            sim.config.adherent.num_heterodoxy_bins,
            sim.config.adherent.num_age_bins,
        );

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
            let age = sampled_age.round().clamp(0.0, oldest_starting_age as f64) as usize;

            // bin this adherent, add to histogram
            root_adherents
                .bin(
                    heterodoxy.value(),
                    age,
                    sim.config.adherent.num_heterodoxy_bins,
                    sim.config.adherent.num_age_bins,
                    sim.config.adherent.max_age_yrs,
                )
                .with_context(|| {
                    format!(
                        "binning initial adherent: heterodoxy={}, age={}",
                        heterodoxy.value(),
                        age
                    )
                })?;
        }

        let root_religion = Religion::new(None, sim.current_year, &mut sim.rng, root_adherents)
            .context("creating new religion")?;

        let _ = sim.active_religions.insert(root_religion);

        Ok(sim)
    }

    pub fn total_population(&self) -> u64 {
        self.active_religions.values().map(|r| r.total_population()).sum()
    }

    pub fn run(&mut self) -> Result<()> {
        for generation in 0..self.config.world.num_generations {
            eprintln!("gen {generation}");
            self.tick()?;
        }

        eprintln!("simulation ended.");

        let final_readout =
            self.build_generation_readout(&Default::default(), self.mean_living_heterodoxy());

        let readout_json =
            serde_json::to_string_pretty(&final_readout).context("serializing final readout")?;
        println!("{readout_json}");

        Ok(())
    }
}
