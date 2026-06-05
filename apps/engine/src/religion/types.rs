use rand::{RngExt, rngs::SmallRng};
use slotmap::new_key_type;

use crate::probability::{UnitInterval, flip_weighted_coin};

new_key_type! {
    pub struct ReligionKey;
    pub struct AdherentKey;
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum AdherentStatus {
    Dead,
    Alive,
}

/// bare minimum container representing adherent of a faith. can add more fields later
pub struct Adherent {
    /// likelihood / tendancy for this person to question current religion
    pub heterodoxy: UnitInterval,

    pub age: u8,

    /// whether they're dead or alive. soft delete
    pub status: AdherentStatus,

    /// religion adherent follows
    pub religion: ReligionKey,
}

impl Adherent {
    const CONVERSION_BASE_RATE: UnitInterval = UnitInterval::new(0.02);
    const HETERODOXY_CHANGE_BASE_RATE: UnitInterval = UnitInterval::new(0.01);
    const AGE_0_TO_49_MORTALITY_RATE: UnitInterval = UnitInterval::new(0.001);
    const AGE_50_TO_69_MORTALITY_RATE: UnitInterval = UnitInterval::new(0.01);
    const AGE_70_TO_79_MORTALITY_RATE: UnitInterval = UnitInterval::new(0.05);
    const AGE_80_PLUS_MORTALITY_RATE: UnitInterval = UnitInterval::new(0.15);
    const AGE_0_TO_12_BIRTH_RATE: UnitInterval = UnitInterval::new(0.0);
    const AGE_13_TO_17_BIRTH_RATE: UnitInterval = UnitInterval::new(0.02);
    const AGE_18_TO_25_BIRTH_RATE: UnitInterval = UnitInterval::new(0.12);
    const AGE_26_TO_35_BIRTH_RATE: UnitInterval = UnitInterval::new(0.16);
    const AGE_36_TO_45_BIRTH_RATE: UnitInterval = UnitInterval::new(0.06);
    const AGE_46_PLUS_BIRTH_RATE: UnitInterval = UnitInterval::new(0.0);

    pub fn gave_birth(&self, rng: &mut SmallRng) -> bool {
        if self.is_dead() {
            return false;
        }

        let birth_rate = match self.age {
            0..=12 => Self::AGE_0_TO_12_BIRTH_RATE,
            13..=17 => Self::AGE_13_TO_17_BIRTH_RATE,
            18..=25 => Self::AGE_18_TO_25_BIRTH_RATE,
            26..=35 => Self::AGE_26_TO_35_BIRTH_RATE,
            36..=45 => Self::AGE_36_TO_45_BIRTH_RATE,
            _ => Self::AGE_46_PLUS_BIRTH_RATE,
        };

        flip_weighted_coin(birth_rate, rng)
    }

    pub fn new(religion: ReligionKey) -> Self {
        Self {
            heterodoxy: UnitInterval::new(0.05),
            age: 0,
            status: AdherentStatus::Alive,
            religion,
        }
    }

    /// called when new sect is made, should this person join?
    fn should_convert(&self, rng: &mut SmallRng) -> bool {
        let probability = Self::CONVERSION_BASE_RATE * self.heterodoxy;

        flip_weighted_coin(probability, rng)
    }

    pub fn try_conversion(&mut self, new_religion: ReligionKey, rng: &mut SmallRng) -> bool {
        if self.should_convert(rng) {
            self.religion = new_religion;
            return true;
        }

        false
    }

    /// per tick, how should this adherent's heterodoxy change
    /// multiply change base rate by their heterodoxy. in other words, more heterodox ppl are more likely to get more heterodox
    fn update_heterodoxy(&mut self, rng: &mut SmallRng) {
        // younger ppl more likely to get more heterodox, old ppl more likely to get less heterodox
        let change = Self::HETERODOXY_CHANGE_BASE_RATE * self.heterodoxy;

        if flip_weighted_coin(self.heterodoxy, rng) {
            self.heterodoxy += change;
        } else {
            self.heterodoxy -= change;
        }
    }

    fn should_die(&mut self, rng: &mut SmallRng) -> bool {
        let mortality = match self.age {
            0..=49 => Self::AGE_0_TO_49_MORTALITY_RATE,
            50..=69 => Self::AGE_50_TO_69_MORTALITY_RATE,
            70..=79 => Self::AGE_70_TO_79_MORTALITY_RATE,
            _ => Self::AGE_80_PLUS_MORTALITY_RATE,
        };

        flip_weighted_coin(mortality, rng)
    }

    pub fn is_dead(&self) -> bool {
        self.status == AdherentStatus::Dead
    }

    /// DON'T FORGET TO NOT CHANGE DEADs
    pub fn update(&mut self, rng: &mut SmallRng) {
        // increment age
        self.age += 1;

        if self.should_die(rng) {
            self.status = AdherentStatus::Dead;
        } else {
            // increment heter
            self.update_heterodoxy(rng);
        }
    }
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
