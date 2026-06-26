use std::collections::HashMap;

use anyhow::Result;
use rand_distr::Beta;

use crate::{
    histogram::HeterodoxyBin, probability::create_child_heterodoxy_distribution,
    religion::ReligionKey, simulation::Simulation,
};

impl Simulation {
    pub(super) fn create_child_heterodoxy_distributions(
        &mut self,
    ) -> Result<HashMap<ReligionKey, Vec<Beta<f64>>>> {
        let mut distr_map: HashMap<ReligionKey, Vec<Beta<f64>>> = HashMap::new();

        for (key, religion) in self.active_religions.iter() {
            match religion {
                crate::religion::Religion::Active { adherents, .. } => {
                    let mean_heterodoxy = adherents.mean_heterodoxy();

                    let mut beta_vec = vec![];

                    // size of heterodoxy col?
                    let length = self.config.adherent.num_heterodoxy_bins + 1;

                    for i in 0..=length {
                        let het_bin = HeterodoxyBin::from(i);
                        let het_distr = create_child_heterodoxy_distribution(
                            het_bin.to_heterodoxy(self.config.adherent.num_heterodoxy_bins),
                            mean_heterodoxy,
                            &self.config.adherent,
                        )?;

                        beta_vec.insert(i, het_distr);
                    }

                    distr_map.insert(key, beta_vec);
                }
                crate::religion::Religion::Extinct { .. } => {}
            }
        }

        Ok(distr_map)
    }
}
