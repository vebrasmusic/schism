use anyhow::{Context, Result};
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
    /// world clock in years, counting up from 0; advanced one generation per tick
    current_year: u32,
}

impl Simulation {
    pub fn new(config: SimulationConfig) -> Result<Self> {
        let mut sim = Self {
            religions: SlotMap::with_key(),
            adherents: SlotMap::with_key(),
            rng: SmallRng::seed_from_u64(config.world.seed),
            config,
            current_year: 0,
        };

        let root_religion = Religion::new(None, sim.current_year, &mut sim.rng)
            .context("creating new religion")?;

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
        // progress chatter goes to stderr so stdout carries only the final
        // readout — that's what lets a caller do `engine run > out.json` and get
        // a clean end-of-run tree with no log lines mixed in.
        let mut final_generation_readout = None;

        for generation in 0..self.config.world.num_generations {
            eprintln!("on generation {generation}");
            final_generation_readout = Some(self.tick()?);
        }

        eprintln!("simulation ended.");

        // emit the world state once, at the very end, as the json "end tree".
        // a zero-generation run never ticks, so fall back to the initial world.
        let final_generation_readout = final_generation_readout.unwrap_or_else(|| {
            let religions_at_start = self.religions.keys().collect();
            self.build_generation_readout(&religions_at_start, self.mean_living_heterodoxy())
        });

        let readout_json = serde_json::to_string_pretty(&final_generation_readout)
            .context("serializing final generation readout")?;
        println!("{readout_json}");

        // throwaway (CDK experiment): also ship the readout to S3 when
        // SCHISM_S3_BUCKET is set. no-op otherwise. delete this line with the
        // `output` module.
        crate::output::upload_if_configured(&readout_json)?;

        Ok(())
    }

    /// mean heterodoxy across the living population. the per-tick `retain` drops
    /// the dead before this is called and the initial population starts alive, so
    /// every current adherent counts.
    fn mean_living_heterodoxy(&self) -> f64 {
        self.adherents
            .iter()
            .map(|(_, adherent)| adherent.heterodoxy.value())
            .sum::<f64>()
            / self.adherents.len() as f64
    }
}
