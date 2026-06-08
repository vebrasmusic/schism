use std::any;

use anyhow::{Context, Result};
use rand::rngs::SmallRng;
use rand_distr::{Beta, Distribution};
use slotmap::new_key_type;

use crate::config::AdherentConfig;
use crate::probability::{UnitInterval, create_child_heterodoxy_distribution, flip_weighted_coin};
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
    pub fn try_birth(
        &self,
        population_mean_heterodoxy: f64,
        config: &AdherentConfig,
        rng: &mut SmallRng,
    ) -> Result<Option<Adherent>> {
        if self.status == AdherentStatus::Dead {
            return Ok(None);
        }

        let birth_rate = config.birth_rate(self.age);

        if !flip_weighted_coin(birth_rate, rng) {
            return Ok(None);
        }

        let distr = create_child_heterodoxy_distribution(self, population_mean_heterodoxy, config)
            .context("tried creating child het distr")?;

        let heterodoxy = UnitInterval::new(distr.sample(rng));

        Ok(Some(Adherent::new(self.religion, heterodoxy, None)))
    }

    /// takes in religion and the distr. descrbing new adherent's hterorodxy.
    /// for children, would be the child one, otherwise the pop. level one.
    /// `age` is optional — newborns pass `None` (age 0); the initial population
    /// passes `Some(_)` with an age sampled from the population distribution.
    pub fn new(religion: ReligionKey, heterodoxy: UnitInterval, age: Option<u8>) -> Self {
        Self {
            heterodoxy,
            age: age.unwrap_or(0),
            status: AdherentStatus::Alive,
            religion,
        }
    }

    /// called when new sect is made, should this person join?
    fn should_convert(
        &self,
        schism_threshold: UnitInterval,
        config: &AdherentConfig,
        rng: &mut SmallRng,
    ) -> bool {
        // a schism splits along the belief fault line: only the heterodox wing —
        // members above the same high-heterodoxy threshold that drove the split —
        // are candidates to break away. the orthodox majority stays put.
        if self.heterodoxy <= schism_threshold {
            return false;
        }

        // within that wing, leaving is still a coin flip weighted by how
        // heterodox they are, so the most heterodox follow the new sect most
        // reliably while the merely-doubtful sometimes stay.
        let probability = self.heterodoxy * config.conversion_base_rate;

        flip_weighted_coin(probability, rng)
    }

    pub fn try_conversion(
        &mut self,
        new_religion: ReligionKey,
        schism_threshold: UnitInterval,
        config: &AdherentConfig,
        rng: &mut SmallRng,
    ) -> bool {
        if self.should_convert(schism_threshold, config, rng) {
            self.religion = new_religion;
            return true;
        }

        false
    }

    /// /// /// per tick, how should this adherent's heterodoxy change
    /// /// /// multiply change base rate by their heterodoxy. in other words, more heterodox ppl are more likely to get more heterodox
    /// fn update_heterodoxy(&mut self, config: &AdherentConfig, rng: &mut SmallRng) {
    ///     // younger ppl more likely to get more heterodox, old ppl more likely to get less heterodox
    ///     let change = self.heterodoxy * config.heterodoxy_change_base_rate;
    ///
    ///     if flip_weighted_coin(self.heterodoxy, rng) {
    ///         self.heterodoxy += change;
    ///     } else {
    ///         self.heterodoxy -= change;
    ///     }
    /// }

    fn should_die(&mut self, config: &AdherentConfig, rng: &mut SmallRng) -> bool {
        // hard cap: at or beyond `max_age_yrs`, survival is treated as
        // impossible. below it, fall back to the actuarial mortality table.
        // without the cap the 20-yr age jumps walk past the oldest band (where
        // mortality reads as ~0) and `age` (a u8) eventually overflows.
        if self.age >= config.max_age_yrs {
            return true;
        }

        let mortality = config.mortality_rate(self.age);

        flip_weighted_coin(mortality, rng)
    }

    pub fn is_dead(&self) -> bool {
        self.status == AdherentStatus::Dead
    }

    /// DON'T FORGET TO NOT CHANGE DEADs
    pub fn update(&mut self, config: &AdherentConfig, rng: &mut SmallRng) {
        // saturating so a large generation length can't overflow `age` (u8); the
        // mortality cap kills anyone this pushes past `max_age_yrs` anyway.
        self.age = self.age.saturating_add(config.generation_length_yrs);

        if self.should_die(config, rng) {
            self.status = AdherentStatus::Dead;
        }
        // } else {
        //     // increment heter
        //     self.update_heterodoxy(config, rng);
        // }
    }
}
