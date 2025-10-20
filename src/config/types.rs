// std imports
use std::ops::{Add, Div, Mul, MulAssign, Sub};

// third-party imports
use serde::Deserialize;

// sub-modules
pub mod dimension;
pub mod dimension_with_initial;
pub mod range;
pub mod snap;
pub mod stepped_range;

// re-exports
pub use dimension::Dimension;
pub use dimension_with_initial::DimensionWithInitial;

/// This type is needed to workaround issues with loading integer types as float in TOML format.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(untagged)]
pub enum Number {
    Int(u16),
    Float(f32),
}

impl Number {
    /// Converts the `Number` to an `f32`.
    pub fn f32(self) -> f32 {
        self.into()
    }
}

impl Default for Number {
    /// Returns the default value for `Number`, which is `Float(0.0)`.
    fn default() -> Self {
        Self::Float(0.0)
    }
}

impl std::fmt::Display for Number {
    /// Formats the `Number` for display.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Int(value) => write!(f, "{value}"),
            Number::Float(value) => write!(f, "{value}"),
        }
    }
}

impl From<Number> for f32 {
    /// Converts a `Number` to an `f32`.
    fn from(value: Number) -> Self {
        match value {
            Number::Int(i) => i as f32,
            Number::Float(f) => f,
        }
    }
}

impl From<f32> for Number {
    /// Converts an `f32` to a `Number`.
    fn from(value: f32) -> Self {
        Number::Float(value)
    }
}

impl Add for Number {
    type Output = f32;

    /// Adds two `Number` values and returns the result as an `f32`.
    fn add(self, rhs: Number) -> Self::Output {
        self + f32::from(rhs)
    }
}

impl Add<f32> for Number {
    type Output = f32;

    /// Adds a `Number` and an `f32` and returns the result as an `f32`.
    fn add(self, rhs: f32) -> Self::Output {
        f32::from(self) + rhs
    }
}

impl Add<Number> for f32 {
    type Output = f32;

    /// Adds an `f32` and a `Number` and returns the result as an `f32`.
    fn add(self, rhs: Number) -> Self::Output {
        self + f32::from(rhs)
    }
}

impl Sub<f32> for Number {
    type Output = f32;

    /// Subtracts an `f32` from a `Number` and returns the result as an `f32`.
    fn sub(self, rhs: f32) -> Self::Output {
        f32::from(self) - rhs
    }
}

impl Sub<Number> for f32 {
    type Output = f32;

    /// Subtracts a `Number` from an `f32` and returns the result as an `f32`.
    fn sub(self, rhs: Number) -> Self::Output {
        self - f32::from(rhs)
    }
}

impl Mul<f32> for Number {
    type Output = f32;

    /// Multiplies a `Number` by an `f32` and returns the result as an `f32`.
    fn mul(self, rhs: f32) -> Self::Output {
        f32::from(self) * rhs
    }
}

impl Mul<Number> for f32 {
    type Output = f32;

    /// Multiplies an `f32` by a `Number` and returns the result as an `f32`.
    fn mul(self, rhs: Number) -> Self::Output {
        self * f32::from(rhs)
    }
}

impl Div<f32> for Number {
    type Output = f32;

    /// Divides a `Number` by an `f32` and returns the result as an `f32`.
    fn div(self, rhs: f32) -> Self::Output {
        f32::from(self) / rhs
    }
}

impl Div<Number> for f32 {
    type Output = f32;

    /// Divides an `f32` by a `Number` and returns the result as an `f32`.
    fn div(self, rhs: Number) -> Self::Output {
        self / f32::from(rhs)
    }
}

impl MulAssign<f32> for Number {
    /// Multiplies the current `Number` by an `f32` and updates its value.
    fn mul_assign(&mut self, rhs: f32) {
        *self = Self::Float(*self * rhs);
    }
}
