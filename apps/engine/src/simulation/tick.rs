use super::Simulation;
use super::readout::GenerationReadout;
use crate::religion::ReligionKey;
use crate::simulation::SimulationScale;
use anyhow::Result;
use std::collections::HashSet;

impl Simulation {
    /// advance the world one generation and return the snapshot describing the
    /// resulting state. the readout is built but not serialized here — the run
    /// loop keeps the final generation's and emits it once at the end.
    pub(super) fn tick(&mut self) -> Result<GenerationReadout> {
        match self.scale {
            SimulationScale::Individual => {
                if self.total_population() > self.config.world.cohort_scale_threshold as u64 {
                    self.scale = SimulationScale::Cohort;
                }
            }
            SimulationScale::Cohort => {}
            SimulationScale::Aggregate => {}
        }

        // snapshot which religions exist before this tick, so the readout can
        // flag any that get born this generation. read-only, doesn't affect sim.
        let religions_at_start: HashSet<ReligionKey> = self.religions.keys().collect();

        // advance the world clock one generation. religions born this tick are
        // stamped with this year, and any that die are stamped extinct with it.
        self.current_year += self.config.adherent.generation_length_yrs as u32;
        let current_year = self.current_year;

        // total living population at the start of the tick, before anyone dies
        // or is born — drives the density-dependent mortality in remove_dead.
        let population_at_tick_start = self.total_population();

        // get rid of any adherents that died
        self.remove_dead(population_at_tick_start)?;

        // advance ages
        self.increment_age()?;

        // add births
        self.add_births()?;

        // check and mark any dead religions
        self.mark_extinct_religions(current_year);

        // now next steps will only touch current living religions

        Ok(self.build_generation_readout(&religions_at_start, self.mean_living_heterodoxy()))
    }
}
