use anyhow::{Context, Result};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use rand_distr::Distribution;
use serde::Serialize;
use slotmap::SlotMap;

use crate::{
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

pub use readout::{ReadoutTotals, ReligionReadout, ReligionStatus, SimulationReadout};

pub enum SimulationScale {
    Individual,
    Cohort,
    #[allow(dead_code)]
    Aggregate,
}

pub struct Simulation {
    active_religions: SlotMap<ReligionKey, Religion>,
    extinct_religions: Vec<(ReligionKey, Religion)>,
    config: SimulationConfig,
    scale: SimulationScale,
    rng: SmallRng,
    seed: u64,
    /// world clock in years, counting up from 0; advanced one generation per tick
    current_year: u32,
}

#[derive(Debug, Serialize)]
struct EngineRunOutput {
    seed: u64,
    readout: SimulationReadout,
}

impl Simulation {
    pub fn new(mut config: SimulationConfig) -> Result<Self> {
        // resolve the chosen environment's tunables (carrying capacity, ...) and
        // fold them into the config so the rest of the engine reads them off
        // `self.config.environment`, same as the other sub-configs.
        config.environment = config.world.environment.config();
        let seed = config.world.seed.unwrap_or_else(|| rand::rng().random());

        let mut sim = Self {
            active_religions: SlotMap::with_key(),
            extinct_religions: Vec::new(),
            rng: SmallRng::seed_from_u64(seed),
            seed,
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
        self.active_religions
            .values()
            .map(|r| r.total_population())
            .sum()
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn run_to_readout(&mut self) -> Result<SimulationReadout> {
        self.run_to_readout_with_progress(|_, _| {})
    }

    pub fn run_to_readout_with_progress(
        &mut self,
        mut on_generation_complete: impl FnMut(u32, u32),
    ) -> Result<SimulationReadout> {
        let total_generations = self.config.world.num_generations;

        for generation in 0..total_generations {
            self.tick()?;
            on_generation_complete(generation + 1, total_generations);
        }

        Ok(self.build_simulation_readout())
    }

    pub fn run(&mut self) -> Result<()> {
        let final_readout = self.run_to_readout()?;
        let output = EngineRunOutput {
            seed: self.seed(),
            readout: final_readout,
        };
        let readout_json =
            serde_json::to_string_pretty(&output).context("serializing final readout")?;
        println!("{readout_json}");

        Ok(())
    }
}
