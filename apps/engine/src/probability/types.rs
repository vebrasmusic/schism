use std::ops::{AddAssign, Mul, SubAssign};

/// a float guaranteed to be between 0 and 1
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct UnitInterval(f32);

impl UnitInterval {
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0 && value <= 1.0);

        Self(value)
    }

    pub const fn value(self) -> f32 {
        self.0
    }
}

impl Mul for UnitInterval {
    type Output = UnitInterval;

    fn mul(self, rhs: Self) -> Self::Output {
        UnitInterval::new(self.0 * rhs.0)
    }
}
impl Mul<f32> for UnitInterval {
    type Output = UnitInterval;

    fn mul(self, rhs: f32) -> Self::Output {
        UnitInterval::new(self.0 * rhs)
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
