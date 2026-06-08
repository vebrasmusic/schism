mod lists;

use lists::{RELIGION_BASE_NAMES, RELIGION_MODIFIERS};
use rand::{rngs::SmallRng, seq::IndexedRandom};

pub fn generate_name(parent_name: Option<&str>, rng: &mut SmallRng) -> String {
    match parent_name {
        None => root_name(rng),
        Some(name) => derivative_name(name, rng),
    }
}

fn root_name(rng: &mut SmallRng) -> String {
    RELIGION_BASE_NAMES
        .choose(rng)
        .copied()
        .unwrap()
        .to_string()
}

fn derivative_name(parent_name: &str, rng: &mut SmallRng) -> String {
    let adj = RELIGION_MODIFIERS.choose(rng).copied().unwrap();

    format!("{adj} {parent_name}")
}
