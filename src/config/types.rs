// std imports
use std::ops::{Add, Div, Mul, Sub};

// third-party imports
use serde::Deserialize;

// This type is needed to workaround issues with loading integer types as float in TOML format.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(untagged)]
pub enum Number {
    Int(u16),
    Float(f32),
}

impl Number {
    pub fn f32(self) -> f32 {
        self.into()
    }
}

impl Default for Number {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Int(i) => write!(f, "{}", i),
            Number::Float(fl) => write!(f, "{}", fl),
        }
    }
}

impl From<Number> for f32 {
    fn from(value: Number) -> Self {
        match value {
            Number::Int(i) => i as f32,
            Number::Float(f) => f,
        }
    }
}

impl From<f32> for Number {
    fn from(value: f32) -> Self {
        Number::Float(value)
    }
}

impl Add for Number {
    type Output = f32;

    fn add(self, rhs: Number) -> Self::Output {
        self + f32::from(rhs)
    }
}

impl Add<f32> for Number {
    type Output = f32;

    fn add(self, rhs: f32) -> Self::Output {
        f32::from(self) + rhs
    }
}

impl Add<Number> for f32 {
    type Output = f32;

    fn add(self, rhs: Number) -> Self::Output {
        self + f32::from(rhs)
    }
}

impl Sub<f32> for Number {
    type Output = f32;

    fn sub(self, rhs: f32) -> Self::Output {
        f32::from(self) - rhs
    }
}

impl Sub<Number> for f32 {
    type Output = f32;

    fn sub(self, rhs: Number) -> Self::Output {
        self - f32::from(rhs)
    }
}

impl Mul<f32> for Number {
    type Output = f32;

    fn mul(self, rhs: f32) -> Self::Output {
        f32::from(self) * rhs
    }
}

impl Mul<Number> for f32 {
    type Output = f32;

    fn mul(self, rhs: Number) -> Self::Output {
        self * f32::from(rhs)
    }
}

impl Div<f32> for Number {
    type Output = f32;

    fn div(self, rhs: f32) -> Self::Output {
        f32::from(self) / rhs
    }
}

impl Div<Number> for f32 {
    type Output = f32;

    fn div(self, rhs: Number) -> Self::Output {
        self / f32::from(rhs)
    }
}
