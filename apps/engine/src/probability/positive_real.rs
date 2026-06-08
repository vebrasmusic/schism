use std::ops::{AddAssign, Div, Mul};

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// a finite float guaranteed to be strictly positive — the open interval (0, ∞)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PositiveReal(f64);

impl PositiveReal {
    /// Construct from a known-good value. Panics if not strictly positive and
    /// finite, so this is for compile-time literals and already-checked results
    /// — untrusted input (config files) goes through `Deserialize`, which errors
    /// instead.
    pub const fn new(value: f64) -> Self {
        assert!(value > 0.0 && value.is_finite());

        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

impl Serialize for PositiveReal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for PositiveReal {
    /// Read a plain number and validate the (0, ∞) invariant here, so a bad
    /// value in a config file surfaces as a deserialization error rather than a
    /// panic — and every downstream consumer can take a `PositiveReal` directly.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;

        if value > 0.0 && value.is_finite() {
            Ok(Self(value))
        } else {
            Err(DeError::custom(format!(
                "value {value} is not a positive real (0.0, ∞)"
            )))
        }
    }
}

impl Mul for PositiveReal {
    type Output = PositiveReal;

    /// closed over (0, ∞): the product of two positives is positive
    fn mul(self, rhs: Self) -> Self::Output {
        PositiveReal::new(self.0 * rhs.0)
    }
}

impl Div for PositiveReal {
    type Output = PositiveReal;

    /// closed over (0, ∞): the quotient of two positives is positive
    fn div(self, rhs: Self) -> Self::Output {
        PositiveReal::new(self.0 / rhs.0)
    }
}

impl AddAssign for PositiveReal {
    /// closed over (0, ∞): the sum of two positives is positive
    fn add_assign(&mut self, rhs: Self) {
        *self = PositiveReal::new(self.0 + rhs.0);
    }
}
