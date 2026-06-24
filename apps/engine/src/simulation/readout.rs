use std::collections::HashSet;

use serde::Serialize;

use crate::religion::ReligionKey;

use super::Simulation;

/// the full world snapshot for a generation. `Serialize` is what gives us the
/// pretty, one-field-per-line output via `to_string_pretty`. owns its strings
/// (rather than borrowing from `Simulation`) so the run loop can hold onto the
/// final generation's readout after the borrow ends and serialize it once at the
/// very end of the run.
#[derive(Serialize)]
pub(crate) struct GenerationReadout {
    totals: Totals,
    religions: Vec<ReligionRow>,
}

#[derive(Serialize)]
struct Totals {
    people: String,
    religions: usize,
    active: usize,
    extinct: usize,
    new_this_generation: usize,
    mean_heterodoxy: f64,
}

#[derive(Serialize)]
struct ReligionRow {
    name: String,
    adherents: String,
    #[serde(skip)]
    adherents_count: usize,
    status: &'static str,
    founding_date: u32,
    extinction_date: Option<u32>,
    age: String,
    parent: String,
    new: bool,
}

fn fmt_count(n: usize) -> String {
    let s = n.to_string();
    let with_commas = s
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(",");
    let suffix = if n >= 1_000_000_000_000_000_000 {
        " (quintillion)"
    } else if n >= 1_000_000_000_000_000 {
        " (quadrillion)"
    } else if n >= 1_000_000_000_000 {
        " (trillion)"
    } else if n >= 1_000_000_000 {
        " (billion)"
    } else if n >= 1_000_000 {
        " (million)"
    } else if n >= 1_000 {
        " (thousand)"
    } else {
        ""
    };
    format!("{with_commas}{suffix}")
}

impl Simulation {
    /// build the detailed snapshot of the world state for a generation. purely
    /// a readout — touches no simulation state. the caller owns the result and
    /// decides when to serialize it (we only emit json once, at end of run).
    pub(super) fn build_generation_readout(
        &self,
        religions_at_start: &HashSet<ReligionKey>,
        mean_population_heterodoxy: f64,
    ) -> GenerationReadout {
        // tally living adherents straight from each religion's histogram, which
        // only ever holds living members (the dead are decremented out per tick).
        let total_people = self.total_population() as usize;

        // one row per religion, newest foundings on top so the freshest sects
        // scan first; biggest congregation breaks ties within a founding cohort.
        let mut religion_rows: Vec<ReligionRow> = self
            .religions
            .iter()
            .map(|(religion_id, religion)| {
                let living_adherents = religion.total_population() as usize;
                let is_new_this_generation = !religions_at_start.contains(&religion_id);
                let parent_name = religion
                    .parent
                    .and_then(|parent_id| self.religions.get(parent_id))
                    .map(|parent| parent.name.as_str())
                    .unwrap_or("none");

                ReligionRow {
                    name: religion.name.clone(),
                    adherents: fmt_count(living_adherents),
                    adherents_count: living_adherents,
                    status: if religion.is_extinct() {
                        "extinct"
                    } else {
                        "active"
                    },
                    founding_date: religion.founding_date,
                    extinction_date: religion.extinction_date(),
                    age: fmt_count(religion.age(self.current_year) as usize),
                    parent: parent_name.to_owned(),
                    new: is_new_this_generation,
                }
            })
            .collect();
        religion_rows.sort_by(|left, right| {
            right
                .founding_date
                .cmp(&left.founding_date)
                .then(right.adherents_count.cmp(&left.adherents_count))
        });

        let total_religions = religion_rows.len();
        let active_religions = religion_rows
            .iter()
            .filter(|row| row.status == "active")
            .count();
        let new_religions = religion_rows.iter().filter(|row| row.new).count();

        GenerationReadout {
            totals: Totals {
                people: fmt_count(total_people),
                religions: total_religions,
                active: active_religions,
                extinct: total_religions - active_religions,
                new_this_generation: new_religions,
                mean_heterodoxy: mean_population_heterodoxy,
            },
            religions: religion_rows,
        }
    }

    /// mean heterodoxy across every living adherent in the world, weighting each
    /// histogram bin's representative heterodoxy value by how many people sit in
    /// it. returns 0.0 for an empty world to avoid dividing by zero.
    pub(super) fn mean_living_heterodoxy(&self) -> f64 {
        let num_heterodoxy_bins = self.config.adherent.num_heterodoxy_bins as f64;
        let mut weighted_heterodoxy_sum = 0.0;
        let mut total_living = 0u64;

        for religion in self.religions.values() {
            for (_age_band, heterodoxy_counts) in religion.adherents.iter_bands() {
                for (heterodoxy_bin, count) in heterodoxy_counts {
                    let heterodoxy_value = heterodoxy_bin.value() as f64 / num_heterodoxy_bins;
                    weighted_heterodoxy_sum += heterodoxy_value * count.value() as f64;
                    total_living += count.value();
                }
            }
        }

        if total_living == 0 {
            0.0
        } else {
            weighted_heterodoxy_sum / total_living as f64
        }
    }
}
