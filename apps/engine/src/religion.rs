use rand::rngs::SmallRng;
use slotmap::new_key_type;

use crate::adherent::Adherent;
use crate::config::ReligionConfig;
use crate::probability::{UnitInterval, flip_weighted_coin};

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
    pub fn new(parent: Option<(&Religion, ReligionKey)>) -> Self {
        match parent {
            None => Self {
                name: Self::generate_name(None),
                age: 0,
                status: ReligionStatus::Active,
                parent: None,
            },
            Some((parent, parent_id)) => Self {
                name: Self::generate_name(Some(&parent.name)),
                age: 0,
                status: ReligionStatus::Active,
                parent: Some(parent_id),
            },
        }
    }

    fn generate_name(parent_name: Option<&str>) -> String {
        match parent_name {
            Some(name) => format!("True {}", name),
            None => "Church of Orange".to_owned(),
        }
    }

    pub fn should_schism(
        &self,
        adherents: &[&Adherent],
        config: &ReligionConfig,
        rng: &mut SmallRng,
    ) -> bool {
        if adherents.len() < config.min_congregation {
            return false;
        }

        let avg_heterodoxy = adherents
            .iter()
            .map(|adherent| adherent.heterodoxy.value())
            .sum::<f64>()
            / adherents.len() as f64;

        let high_heterodoxy_share = adherents
            .iter()
            .filter(|adherent| adherent.heterodoxy.value() > config.high_heterodoxy_threshold)
            .count() as f64
            / adherents.len() as f64;

        let population_factor =
            (adherents.len() as f64 / config.population_factor_pivot).min(1.0);

        // clamp so a tuned-up base rate can never push us past the [0, 1] invariant
        let chance = (config.schism_base_rate
            * avg_heterodoxy
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
