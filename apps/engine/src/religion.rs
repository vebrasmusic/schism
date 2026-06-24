use anyhow::Result;
use rand::rngs::SmallRng;
use slotmap::{Key, new_key_type};
use std::fmt;
mod naming;

use crate::histogram::PopulationHistogram;
use crate::religion::naming::generate_name;

new_key_type! {
    pub struct ReligionKey;
}

impl fmt::Display for ReligionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "religion:{}", self.data().as_ffi())
    }
}

/// whether a religion is still followed, and if not, when it died out. the
/// `Extinct` variant carries its extinction year so the two states can't drift.
pub enum ReligionStatus {
    Active,
    Extinct(u32),
}

/// modelled as a tree structure
pub struct Religion {
    /// name of the religion
    pub name: String,

    /// world-year the religion was founded
    pub founding_date: u32,

    /// active, or extinct as of a given world-year
    pub status: ReligionStatus,

    /// parent religion node
    pub parent: Option<ReligionKey>,

    /// new population histogram of adherents to this faith
    pub adherents: PopulationHistogram,
}

impl Religion {
    pub fn new(
        parent: Option<(&Religion, ReligionKey)>,
        founding_date: u32,
        rng: &mut SmallRng,
        adherents: PopulationHistogram,
    ) -> Result<Self> {
        let result = match parent {
            None => Self {
                name: generate_name(None, rng),
                founding_date,
                status: ReligionStatus::Active,
                parent: None,
                adherents,
            },
            Some((parent, parent_id)) => Self {
                name: generate_name(Some(&parent.name), rng),
                founding_date,
                status: ReligionStatus::Active,
                parent: Some(parent_id),
                adherents: todo!("migrate some adherents to new religion"),
            },
        };

        Ok(result)
    }

    pub fn mark_extinct(&mut self, extinction_date: u32) {
        self.status = ReligionStatus::Extinct(extinction_date)
    }

    pub fn total_population(&self) -> u64 {
        self.adherents.total()
    }

    pub fn is_extinct(&self) -> bool {
        matches!(self.status, ReligionStatus::Extinct(_))
    }

    /// world-year the religion died out, or `None` while still followed
    pub fn extinction_date(&self) -> Option<u32> {
        match self.status {
            ReligionStatus::Active => None,
            ReligionStatus::Extinct(extinction_date) => Some(extinction_date),
        }
    }

    /// age in years: founding to extinction if extinct, else founding to the
    /// current world-year.
    pub fn age(&self, current_year: u32) -> u32 {
        let end_year = self.extinction_date().unwrap_or(current_year);
        end_year.saturating_sub(self.founding_date)
    }
}
