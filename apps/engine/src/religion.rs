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

pub enum Religion {
    Active {
        name: String,
        founding_date: u32,
        parent: Option<ReligionKey>,
        adherents: PopulationHistogram,
    },
    Extinct {
        name: String,
        founding_date: u32,
        parent: Option<ReligionKey>,
        extinction_date: u32,
    },
}

impl Religion {
    pub fn new(
        parent: Option<(&str, ReligionKey)>,
        founding_date: u32,
        rng: &mut SmallRng,
        adherents: PopulationHistogram,
    ) -> Result<Self> {
        let result = match parent {
            None => Self::Active {
                name: generate_name(None, rng),
                founding_date,
                parent: None,
                adherents,
            },
            Some((parent_religion_name, parent_id)) => Self::Active {
                name: generate_name(Some(parent_religion_name), rng),
                founding_date,
                parent: Some(parent_id),
                adherents,
            },
        };

        Ok(result)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Active { name, .. } | Self::Extinct { name, .. } => name,
        }
    }

    pub fn founding_date(&self) -> u32 {
        match self {
            Self::Active { founding_date, .. } | Self::Extinct { founding_date, .. } => {
                *founding_date
            }
        }
    }

    pub fn parent(&self) -> Option<ReligionKey> {
        match self {
            Self::Active { parent, .. } | Self::Extinct { parent, .. } => *parent,
        }
    }

    pub fn mark_extinct(&mut self, extinction_date: u32) {
        let placeholder = Self::Extinct {
            name: String::new(),
            founding_date: 0,
            parent: None,
            extinction_date: 0,
        };
        let current = std::mem::replace(self, placeholder);
        *self = match current {
            Self::Active {
                name,
                founding_date,
                parent,
                ..
            } => Self::Extinct {
                name,
                founding_date,
                parent,
                extinction_date,
            },
            already_extinct => already_extinct,
        };
    }

    pub fn total_population(&self) -> u64 {
        match self {
            Self::Active { adherents, .. } => adherents.total(),
            Self::Extinct { .. } => 0,
        }
    }

    pub fn mean_heterodoxy(&self) -> f64 {
        match self {
            Self::Active { adherents, .. } => adherents.mean_heterodoxy(),
            Self::Extinct { .. } => 0.0,
        }
    }

    pub fn is_extinct(&self) -> bool {
        matches!(self, Self::Extinct { .. })
    }

    pub fn extinction_date(&self) -> Option<u32> {
        match self {
            Self::Active { .. } => None,
            Self::Extinct {
                extinction_date, ..
            } => Some(*extinction_date),
        }
    }

    pub fn age(&self, current_year: u32) -> u32 {
        let end_year = self.extinction_date().unwrap_or(current_year);
        end_year.saturating_sub(self.founding_date())
    }
}
