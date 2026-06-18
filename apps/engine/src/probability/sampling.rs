use std::collections::HashMap;

use rand::{RngExt, rngs::SmallRng};
use rand_distr::num_traits::ToPrimitive;

use crate::{adherent::Adherent, probability::UnitInterval};

/// given some probability (between 0 and 1) tell me if the coin flipped true or false
pub fn flip_weighted_coin(probability: UnitInterval, rng: &mut SmallRng) -> bool {
    rng.random_bool(probability.value())
}
