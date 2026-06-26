use schemars::JsonSchema;
use serde::Serialize;

use crate::religion::{Religion, ReligionKey};

use super::Simulation;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SimulationReadout {
    pub totals: ReadoutTotals,
    pub religions: Vec<ReligionReadout>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ReadoutTotals {
    pub population: u64,
    pub religions: usize,
    pub active: usize,
    pub extinct: usize,
    pub mean_heterodoxy: f64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ReligionReadout {
    pub name: String,
    pub adherents: u64,
    pub status: ReligionStatus,
    pub founding_date: u32,
    pub extinction_date: Option<u32>,
    pub age: u32,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReligionStatus {
    Active,
    Extinct,
}

pub struct ReligionReadoutSource<'a> {
    pub religion: &'a Religion,
    pub current_year: u32,
    pub parent_name: Option<String>,
}

impl From<ReligionReadoutSource<'_>> for ReligionReadout {
    fn from(source: ReligionReadoutSource<'_>) -> Self {
        match source.religion {
            Religion::Active {
                name,
                founding_date,
                parent,
                adherents,
            } => {
                let _parent_id = parent;

                Self {
                    name: name.clone(),
                    adherents: adherents.total(),
                    status: ReligionStatus::Active,
                    founding_date: *founding_date,
                    extinction_date: None,
                    age: source.current_year.saturating_sub(*founding_date),
                    parent: source.parent_name,
                }
            }
            Religion::Extinct {
                name,
                founding_date,
                parent,
                extinction_date,
            } => {
                let _parent_id = parent;

                Self {
                    name: name.clone(),
                    adherents: 0,
                    status: ReligionStatus::Extinct,
                    founding_date: *founding_date,
                    extinction_date: Some(*extinction_date),
                    age: extinction_date.saturating_sub(*founding_date),
                    parent: source.parent_name,
                }
            }
        }
    }
}

impl Simulation {
    pub(super) fn build_simulation_readout(&self) -> SimulationReadout {
        let lookup_parent_name = |parent_id: ReligionKey| -> Option<String> {
            self.active_religions
                .get(parent_id)
                .map(|r| r.name().to_owned())
                .or_else(|| {
                    self.extinct_religions
                        .iter()
                        .find(|(key, _)| *key == parent_id)
                        .map(|(_, religion)| religion.name().to_owned())
                })
        };

        let make_row = |religion: &Religion| {
            ReligionReadout::from(ReligionReadoutSource {
                religion,
                current_year: self.current_year,
                parent_name: religion.parent().and_then(lookup_parent_name),
            })
        };

        let mut religion_rows: Vec<ReligionReadout> = self
            .active_religions
            .values()
            .map(make_row)
            .chain(
                self.extinct_religions
                    .iter()
                    .map(|(_, religion)| make_row(religion)),
            )
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
            .filter(|row| row.status == ReligionStatus::Active)
            .count();

        SimulationReadout {
            totals: ReadoutTotals {
                population: self.total_population(),
                religions: total_religions,
                active: active_religions,
                extinct: total_religions - active_religions,
                mean_heterodoxy: self.mean_living_heterodoxy(),
            },
            religions: religion_rows,
        }
    }

    /// mean heterodoxy across every living adherent in the world, weighting each
    /// histogram bin's representative heterodoxy value by how many people sit in
    /// it. returns 0.0 for an empty world to avoid dividing by zero.
    pub(super) fn mean_living_heterodoxy(&self) -> f64 {
        let num_het_bins = self.config.adherent.num_heterodoxy_bins;
        let mut weighted_heterodoxy_sum = 0.0;
        let mut total_living = 0u64;

        for religion in self.active_religions.values() {
            match religion {
                Religion::Active {
                    name: _name,
                    founding_date: _founding_date,
                    parent: _parent,
                    adherents,
                } => {
                    for (_age_band, heterodoxy_counts) in adherents.iter_bands() {
                        for (heterodoxy_bin, count) in heterodoxy_counts {
                            weighted_heterodoxy_sum +=
                                heterodoxy_bin.to_heterodoxy(num_het_bins) * count.value() as f64;
                            total_living += count.value();
                        }
                    }
                }
                Religion::Extinct {
                    name: _name,
                    founding_date: _founding_date,
                    parent: _parent,
                    extinction_date: _extinction_date,
                } => {}
            }
        }

        if total_living == 0 {
            0.0
        } else {
            weighted_heterodoxy_sum / total_living as f64
        }
    }
}
