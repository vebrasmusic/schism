use rand::{RngExt, rngs::SmallRng};
use slotmap::new_key_type;

use crate::adherent::Adherent;

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
    /// ratio of heterodoxy / orthodoxy in population, over which a new sect is more likely to form
    const ADHERENT_HETERODOXY_THRESHOLD: f32 = 0.3;

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
            Some(name) => format!("{}ism", name),
            None => "Gurneyism".to_owned(),
        }
    }

    pub fn should_schism(&self, adherents: &[&Adherent], rng: &mut SmallRng) -> bool {
        if adherents.len() < 50 {
            return false;
        }

        let avg_heterodoxy = adherents
            .iter()
            .map(|adherent| adherent.heterodoxy.value() as f64)
            .sum::<f64>()
            / adherents.len() as f64;

        let high_heterodoxy_share = adherents
            .iter()
            .filter(|adherent| adherent.heterodoxy.value() as f64 > 0.7)
            .count() as f64
            / adherents.len() as f64;

        let population_factor = (adherents.len() as f64 / 1000.0).min(1.0);

        let chance = 0.01 * avg_heterodoxy * (1.0 + high_heterodoxy_share) * population_factor;

        rng.random_bool(chance)
    }

    pub fn mark_extinct(&mut self) {
        self.status = ReligionStatus::Extinct
    }

    pub fn is_extinct(&self) -> bool {
        self.status == ReligionStatus::Extinct
    }
}
