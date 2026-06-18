use std::collections::HashMap;

use anyhow::Result;
use rand_distr::{Beta, Normal, num_traits::ToPrimitive};

use crate::{adherent::Adherent, config::AdherentConfig};

/// initial spread of starting ages, so the sim doesn't begin with everyone at
/// age 0. samples are real-valued and unbounded, so the caller rounds and clamps
/// each draw into a valid living age (`0..max_age_yrs`).
pub fn create_initial_population_age_distribution(config: &AdherentConfig) -> Result<Normal<f64>> {
    let distr = Normal::new(
        config.population_mean_age_yrs.value(),
        config.population_age_spread_yrs.value(),
    )?;

    Ok(distr)
}

/// initial spread of heterodoxy
pub fn create_initial_population_heterodoxy_distribution(
    config: &AdherentConfig,
) -> Result<Beta<f64>> {
    beta_from_mean_concentration(
        config.population_mean_heterodoxy.value(),
        config.population_heterodoxy_concentration.value(),
    )
}

pub fn bin_adherents(adherents: Vec<&Adherent>, num_bins: usize) -> HashMap<usize, Vec<&Adherent>> {
    let mut bins: HashMap<usize, Vec<&Adherent>> = HashMap::new();

    for adherent in adherents {
        // take the decimal het. value, mult. by num bins and round to get nearest int. bin
        let nearest_bin = (adherent.heterodoxy.value() * num_bins as f64)
            .round()
            .to_usize()
            .unwrap(); // cause we know we round, and it's bounded

        bins.entry(nearest_bin)
            .and_modify(|v| v.push(adherent))
            .or_insert(vec![adherent]);
    }

    bins
}

/// distr. that describes a new child given their parent and societies attributes
pub fn create_child_heterodoxy_distribution(
    parent_mean: f64,
    current_population_mean_heterodoxy: f64,
    config: &AdherentConfig,
) -> Result<Beta<f64>> {
    let mean_child_heterodoxy = config.parental_heterodoxy_influence.value() * parent_mean
        + (1.0 - config.parental_heterodoxy_influence.value()) * current_population_mean_heterodoxy;

    beta_from_mean_concentration(
        mean_child_heterodoxy,
        config.child_heterodoxy_concentration.value(),
    )
}

/// Build a Beta from the mean / concentration parameterization both heterodoxy
/// distributions share: `alpha = mean * concentration`, `beta = (1 - mean) *
/// concentration`.
///
/// `Beta::new` rejects a non-positive shape parameter, so the moment `mean`
/// reaches 0 or 1 the corresponding shape collapses to `0.0` and construction
/// errors out. Heterodoxy genuinely drifts toward 0 over a long run, so an
/// un-clamped mean *will* eventually crash the sim. Clamping `mean` into the
/// open interval `(0, 1)` keeps both shapes strictly positive: the distribution
/// saturates into a spike just shy of the boundary instead of erroring.
fn beta_from_mean_concentration(mean: f64, concentration: f64) -> Result<Beta<f64>> {
    // how far from the {0, 1} endpoints we hold the mean: small enough to leave
    // the dynamics untouched away from the boundary, large enough that `mean *
    // concentration` stays comfortably positive for any sane concentration (no
    // underflow to 0.0).
    const MEAN_EPSILON: f64 = 1e-9;

    // a non-finite mean (e.g. 0/0 from an empty population) has no location to
    // clamp toward, so fall back to the midpoint rather than feed NaN to Beta.
    let bounded_mean = if mean.is_finite() {
        mean.clamp(MEAN_EPSILON, 1.0 - MEAN_EPSILON)
    } else {
        0.5
    };

    let alpha = bounded_mean * concentration;
    let beta = (1.0 - bounded_mean) * concentration;

    let distr = Beta::new(alpha, beta)?;

    Ok(distr)
}
