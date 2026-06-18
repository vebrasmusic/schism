use std::collections::{HashMap, HashSet};

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
    people: usize,
    religions: usize,
    active: usize,
    extinct: usize,
    new_this_generation: usize,
    mean_heterodoxy: f64,
}

#[derive(Serialize)]
struct ReligionRow {
    name: String,
    adherents: usize,
    status: &'static str,
    founding_date: u32,
    extinction_date: Option<u32>,
    age: u32,
    parent: String,
    new: bool,
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
        // tally living adherents per religion, recomputed fresh so it reflects
        // the post-schism / post-conversion state of this generation.
        let mut adherent_counts: HashMap<ReligionKey, usize> = HashMap::new();
        let mut total_people = 0usize;
        for adherent in self.adherents.values() {
            if adherent.is_dead() {
                continue;
            }
            total_people += 1;
            *adherent_counts.entry(adherent.religion).or_default() += 1;
        }

        if total_people > 75_864_062 {
            panic!("population {total_people} exceeds 71 million");
        }

        // one row per religion, newest foundings on top so the freshest sects
        // scan first; biggest congregation breaks ties within a founding cohort.
        let mut religion_rows: Vec<ReligionRow> = self
            .religions
            .iter()
            .map(|(religion_id, religion)| {
                let living_adherents = adherent_counts.get(&religion_id).copied().unwrap_or(0);
                let is_new_this_generation = !religions_at_start.contains(&religion_id);
                let parent_name = religion
                    .parent
                    .and_then(|parent_id| self.religions.get(parent_id))
                    .map(|parent| parent.name.as_str())
                    .unwrap_or("none");

                ReligionRow {
                    name: religion.name.clone(),
                    adherents: living_adherents,
                    status: if religion.is_extinct() {
                        "extinct"
                    } else {
                        "active"
                    },
                    founding_date: religion.founding_date,
                    extinction_date: religion.extinction_date(),
                    age: religion.age(self.current_year),
                    parent: parent_name.to_owned(),
                    new: is_new_this_generation,
                }
            })
            .collect();
        religion_rows.sort_by(|left, right| {
            right
                .founding_date
                .cmp(&left.founding_date)
                .then(right.adherents.cmp(&left.adherents))
        });

        let total_religions = religion_rows.len();
        let active_religions = religion_rows
            .iter()
            .filter(|row| row.status == "active")
            .count();
        let new_religions = religion_rows.iter().filter(|row| row.new).count();

        GenerationReadout {
            totals: Totals {
                people: total_people,
                religions: total_religions,
                active: active_religions,
                extinct: total_religions - active_religions,
                new_this_generation: new_religions,
                mean_heterodoxy: mean_population_heterodoxy,
            },
            religions: religion_rows,
        }
    }
}
