use anyhow::{Context, Result};
use rand::rngs::SmallRng;
use slotmap::new_key_type;
mod naming;

use crate::adherent::Adherent;
use crate::config::ReligionConfig;
use crate::probability::{UnitInterval, flip_weighted_coin};
use crate::religion::naming::generate_name;

new_key_type! {
    pub struct ReligionKey;
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ReligionStatus {
    Active,
    Extinct,
}

/// modelled as a tree structure
pub struct Religion {
    /// name of the religion
    pub name: String,

    /// age in generations of religion
    pub age: u32,

    /// is the relgiion still followed
    pub status: ReligionStatus,

    /// parent religion node
    pub parent: Option<ReligionKey>,
}

impl Religion {
    pub fn new(parent: Option<(&Religion, ReligionKey)>, rng: &mut SmallRng) -> Result<Self> {
        let result = match parent {
            None => Self {
                name: generate_name(None, rng),
                age: 0,
                status: ReligionStatus::Active,
                parent: None,
            },
            Some((parent, parent_id)) => Self {
                name: generate_name(Some(&parent.name), rng),
                age: 0,
                status: ReligionStatus::Active,
                parent: Some(parent_id),
            },
        };

        Ok(result)
    }

    pub fn should_schism(
        &self,
        adherents: &[&Adherent],
        mean_heterodoxy: f64,
        config: &ReligionConfig,
        rng: &mut SmallRng,
    ) -> bool {
        if adherents.len() < config.min_congregation {
            return false;
        }

        let high_heterodoxy_share = adherents
            .iter()
            .filter(|adherent| adherent.heterodoxy > config.high_heterodoxy_threshold)
            .count() as f64
            / adherents.len() as f64;

        let population_factor = (adherents.len() as f64 / config.population_factor_pivot).min(1.0);

        // clamp so a tuned-up base rate can never push us past the [0, 1] invariant
        let chance = (config.schism_base_rate.value()
            * mean_heterodoxy
            * (1.0 + high_heterodoxy_share)
            * population_factor)
            .clamp(0.0, 1.0);

        flip_weighted_coin(UnitInterval::new(chance), rng)
    }

    pub fn mark_extinct(&mut self) {
        self.status = ReligionStatus::Extinct
    }

    pub fn is_extinct(&self) -> bool {
        self.status == ReligionStatus::Extinct
    }
}
