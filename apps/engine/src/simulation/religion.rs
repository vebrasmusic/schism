use crate::simulation::Simulation;

impl Simulation {
    /// total all pop in all religions, check if they're extinct after the whole birth /death cycle
    pub(super) fn mark_extinct_religions(&mut self, current_date: u32) {
        for (_, religion) in &mut self.religions {
            if religion.total_population() == 0 {
                religion.mark_extinct(current_date);
            }
        }
    }
}
