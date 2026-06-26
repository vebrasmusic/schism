use super::Simulation;
use crate::simulation::SimulationScale;
use anyhow::Result;

impl Simulation {
    pub(super) fn tick(&mut self) -> Result<()> {
        match self.scale {
            SimulationScale::Individual => {
                if self.total_population() > self.config.world.cohort_scale_threshold as u64 {
                    self.scale = SimulationScale::Cohort;
                }
            }
            SimulationScale::Cohort => {}
            SimulationScale::Aggregate => {}
        }

        self.current_year += self.config.adherent.generation_length_yrs as u32;
        let current_year = self.current_year;

        let population_at_tick_start = self.total_population();

        self.remove_dead(population_at_tick_start)?;

        if self.total_population() == 0 {
            panic!("ran out of ppl!");
        };

        self.increment_age()?;

        let child_distributions = self.create_child_heterodoxy_distributions()?;

        self.add_births(child_distributions)?;

        self.mark_extinct_religions(current_year);

        self.schism_religions()?;

        Ok(())
    }
}
