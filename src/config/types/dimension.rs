// std imports
use std::{fmt, str::FromStr};

// third-party imports
use serde::Deserialize;
use thiserror::Error;

use super::{
    snap::SnapUp,
    stepped_range::{SteppedRange, SteppedRangeParseError},
};

#[derive(Debug, Error)]
pub enum DimensionParseError<E> {
    #[error(transparent)]
    RangeParseError(#[from] SteppedRangeParseError<E>),
    #[error("Failed to parse dimension value: {0}")]
    ValueParseError(E),
}

/// Dimension enumeration supporting fixed values, auto-sizing, and range constraints.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dimension<T> {
    Auto,
    #[serde(untagged)]
    Limited(SteppedRange<T>),
    #[serde(untagged)]
    Fixed(T),
}

impl<T> Dimension<T>
where
    T: Copy,
{
    pub fn with_min(self, val: T) -> Self {
        Self::Limited(self.range().with_min(val))
    }

    pub fn with_max(self, val: T) -> Self {
        Self::Limited(self.range().with_max(val))
    }

    /// Get the minimum value of the dimension if it exists.
    pub fn min(&self) -> Option<T>
    where
        T: Copy,
    {
        match self {
            Self::Auto => None,
            Self::Limited(range) => range.min(),
            Self::Fixed(value) => Some(*value),
        }
    }

    /// Get the maximum value of the dimension if it exists.
    pub fn max(&self) -> Option<T> {
        match self {
            Self::Auto => None,
            Self::Limited(range) => range.max(),
            Self::Fixed(value) => Some(*value),
        }
    }

    /// Get the step value of the dimension if it exists.
    pub fn step(&self) -> Option<T> {
        match self {
            Self::Auto => None,
            Self::Limited(range) => range.step(),
            Self::Fixed(_) => None,
        }
    }

    /// Get the range of the dimension.
    pub fn range(&self) -> SteppedRange<T> {
        match self {
            Self::Auto => Default::default(),
            Self::Limited(range) => *range,
            Self::Fixed(value) => SteppedRange {
                range: super::range::PartialRange {
                    min: Some(*value),
                    max: Some(*value),
                },
                step: None,
            },
        }
    }
}

impl<T> Dimension<T>
where
    T: PartialOrd + Copy + SnapUp,
{
    /// Fit a value into the dimension constraints.
    pub fn fit(&self, value: T) -> T {
        match self {
            Self::Auto => value,
            Self::Limited(range) => range.fit(value),
            Self::Fixed(value) => *value,
        }
    }
}

impl<T> From<T> for Dimension<T>
where
    T: FromStr + Copy,
    T::Err: std::fmt::Display,
{
    fn from(value: T) -> Self {
        Self::Fixed(value)
    }
}

impl<T> From<SteppedRange<T>> for Dimension<T>
where
    T: FromStr + Copy,
    T::Err: std::fmt::Display,
{
    fn from(range: SteppedRange<T>) -> Self {
        Self::Limited(range)
    }
}

impl<T> FromStr for Dimension<T>
where
    T: FromStr + Copy,
    T::Err: fmt::Display,
{
    type Err = DimensionParseError<T::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("auto") {
            Ok(Self::Auto)
        } else if s.contains("..") {
            let range = SteppedRange::from_str(s)?;
            Ok(Self::Limited(range))
        } else {
            let value = s
                .parse::<T>()
                .map_err(DimensionParseError::ValueParseError)?;
            Ok(Self::Fixed(value))
        }
    }
}

impl<T> fmt::Display for Dimension<T>
where
    T: FromStr + Copy + fmt::Display,
    T::Err: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Limited(range) => write!(f, "{range}"),
            Self::Fixed(value) => write!(f, "{value}"),
        }
    }
}

#[cfg(test)]
mod tests;
