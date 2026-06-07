use std::ops::{AddAssign, Mul, SubAssign};

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// a float guaranteed to be between 0 and 1
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct UnitInterval(f64);

impl UnitInterval {
    /// Construct from a known-good value. Panics if out of range, so this is for
    /// compile-time literals and already-clamped results — untrusted input
    /// (config files) goes through `Deserialize`, which errors instead.
    pub const fn new(value: f64) -> Self {
        assert!(value >= 0.0 && value <= 1.0);

        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

impl Serialize for UnitInterval {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for UnitInterval {
    /// Read a plain number and validate the [0, 1] invariant here, so a bad
    /// value in a config file surfaces as a deserialization error rather than a
    /// panic — and every downstream consumer can take a `UnitInterval` directly.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;

        if (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DeError::custom(format!(
                "value {value} is outside the unit interval [0.0, 1.0]"
            )))
        }
    }
}

impl Mul for UnitInterval {
    type Output = UnitInterval;

    fn mul(self, rhs: Self) -> Self::Output {
        UnitInterval::new(self.0 * rhs.0)
    }
}

impl AddAssign for UnitInterval {
    /// saturates at 1.0 so the [0, 1] invariant always holds
    fn add_assign(&mut self, rhs: Self) {
        *self = UnitInterval::new((self.0 + rhs.0).clamp(0.0, 1.0));
    }
}

impl SubAssign for UnitInterval {
    /// saturates at 0.0 so the [0, 1] invariant always holds
    fn sub_assign(&mut self, rhs: Self) {
        *self = UnitInterval::new((self.0 - rhs.0).clamp(0.0, 1.0));
    }
}
