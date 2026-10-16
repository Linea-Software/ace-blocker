use nutype::nutype;
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};
use utoipa::ToSchema;

#[nutype(
    sanitize(with = |val| val.clamp(0.0, 100.0)),
    derive(
        Debug,
        Display,
        Clone,
        Copy,
        PartialEq,
        PartialOrd,
        Serialize,
        Deserialize,
        JsonSchema,
    ),
    derive_unchecked(ToSchema)
)]
pub struct Percentage(f64);

impl Add for Percentage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Percentage::new(self.into_inner() + other.into_inner())
    }
}

impl AddAssign for Percentage {
    fn add_assign(&mut self, other: Self) {
        *self = Percentage::new(self.into_inner() + other.into_inner());
    }
}

impl Sub for Percentage {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Percentage::new(self.into_inner() - other.into_inner())
    }
}

impl SubAssign for Percentage {
    fn sub_assign(&mut self, other: Self) {
        *self = Percentage::new(self.into_inner() - other.into_inner());
    }
}

impl Mul for Percentage {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        // Divide by 100.0 to correctly scale the resulting percentage
        Percentage::new((self.into_inner() * rhs.into_inner()) / 100.0)
    }
}
