use std::collections::{HashMap, HashSet};

use crate::religion::{Religion, ReligionKey};

use super::Simulation;

impl Simulation {
    /// detailed, json-ish dump of the world state at the end of a generation.
    /// purely a readout — touches no simulation state.
    pub(super) fn print_generation_readout(
        &self,
        religions_at_start: &HashSet<ReligionKey>,
        mean_population_heterodoxy: f64,
    ) {
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

        // one row per religion, biggest congregation first so it scans top-down.
        let mut religion_rows: Vec<(&Religion, usize, bool)> = self
            .religions
            .iter()
            .map(|(religion_id, religion)| {
                let living_adherents = adherent_counts.get(&religion_id).copied().unwrap_or(0);
                let is_new_this_generation = !religions_at_start.contains(&religion_id);
                (religion, living_adherents, is_new_this_generation)
            })
            .collect();
        religion_rows.sort_by(|left, right| right.1.cmp(&left.1));

        let total_religions = religion_rows.len();
        let active_religions = religion_rows
            .iter()
            .filter(|(religion, _, _)| !religion.is_extinct())
            .count();
        let extinct_religions = total_religions - active_religions;
        let new_religions = religion_rows
            .iter()
            .filter(|(_, _, is_new)| *is_new)
            .count();

        println!("{{");
        println!("  \"totals\": {{");
        println!("    \"people\": {total_people},");
        println!("    \"religions\": {total_religions},");
        println!("    \"active\": {active_religions},");
        println!("    \"extinct\": {extinct_religions},");
        println!("    \"new_this_generation\": {new_religions},");
        println!("    \"mean_heterodoxy\": {mean_population_heterodoxy}");
        println!("  }},");
        println!("  \"religions\": [");

        for (index, (religion, living_adherents, is_new)) in religion_rows.iter().enumerate() {
            let parent_name = religion
                .parent
                .and_then(|parent_id| self.religions.get(parent_id))
                .map(|parent| parent.name.as_str())
                .unwrap_or("none");

            let status = if religion.is_extinct() {
                "extinct"
            } else {
                "active"
            };

            let trailing_comma = if index + 1 < religion_rows.len() {
                ","
            } else {
                ""
            };
            let new_marker = if *is_new { "   <-- NEW" } else { "" };

            println!(
                "    {{ \"name\": \"{}\", \"adherents\": {}, \"status\": \"{}\", \"age\": {}, \"parent\": \"{}\", \"new\": {} }}{}{}",
                religion.name,
                living_adherents,
                status,
                religion.age,
                parent_name,
                is_new,
                trailing_comma,
                new_marker,
            );
        }

        println!("  ]");
        println!("}}");
    }
}
