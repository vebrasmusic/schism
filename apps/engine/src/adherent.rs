use rand::rngs::SmallRng;
use slotmap::new_key_type;

use crate::config::AdherentConfig;
use crate::probability::{UnitInterval, flip_weighted_coin};
use crate::religion::ReligionKey;

new_key_type! {
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
    pub fn gave_birth(&self, config: &AdherentConfig, rng: &mut SmallRng) -> bool {
        if self.is_dead() {
            return false;
        }

        let birth_rate = UnitInterval::new(config.birth_rate(self.age));

        flip_weighted_coin(birth_rate, rng)
    }

    pub fn new(religion: ReligionKey, config: &AdherentConfig) -> Self {
        Self {
            heterodoxy: UnitInterval::new(config.starting_heterodoxy),
            age: 0,
            status: AdherentStatus::Alive,
            religion,
        }
    }

    /// called when new sect is made, should this person join?
    fn should_convert(&self, config: &AdherentConfig, rng: &mut SmallRng) -> bool {
        let probability = self.heterodoxy * config.conversion_base_rate;

        flip_weighted_coin(probability, rng)
    }

    pub fn try_conversion(
        &mut self,
        new_religion: ReligionKey,
        config: &AdherentConfig,
        rng: &mut SmallRng,
    ) -> bool {
        if self.should_convert(config, rng) {
            self.religion = new_religion;
            return true;
        }

        false
    }

    /// per tick, how should this adherent's heterodoxy change
    /// multiply change base rate by their heterodoxy. in other words, more heterodox ppl are more likely to get more heterodox
    fn update_heterodoxy(&mut self, config: &AdherentConfig, rng: &mut SmallRng) {
        // younger ppl more likely to get more heterodox, old ppl more likely to get less heterodox
        let change = self.heterodoxy * config.heterodoxy_change_base_rate;

        if flip_weighted_coin(self.heterodoxy, rng) {
            self.heterodoxy += change;
        } else {
            self.heterodoxy -= change;
        }
    }

    fn should_die(&mut self, config: &AdherentConfig, rng: &mut SmallRng) -> bool {
        let mortality = UnitInterval::new(config.mortality_rate(self.age));

        flip_weighted_coin(mortality, rng)
    }

    pub fn is_dead(&self) -> bool {
        self.status == AdherentStatus::Dead
    }

    /// DON'T FORGET TO NOT CHANGE DEADs
    pub fn update(&mut self, config: &AdherentConfig, rng: &mut SmallRng) {
        // increment age
        self.age += 1;

        if self.should_die(config, rng) {
            self.status = AdherentStatus::Dead;
        } else {
            // increment heter
            self.update_heterodoxy(config, rng);
        }
    }
}
