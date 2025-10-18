// standard imports
use std::{fmt, str::FromStr};

// third-party imports
use serde::Deserialize;
use thiserror::Error;

use crate::config::types::snap::SnapUp;

// crate imports
use super::range::{PartialRange, RangeParseError};

#[derive(Debug, Clone, PartialEq, Eq, Copy, Deserialize)]
pub struct SteppedRange<T> {
    #[serde(flatten)]
    pub range: PartialRange<T>,
    pub step: Option<T>,
}

impl<T> SteppedRange<T> {
    /// Create a new `SteppedRange` with the given minimum and maximum bounds and step.
    pub fn new(min: Option<T>, max: Option<T>, step: Option<T>) -> Self {
        Self {
            range: PartialRange { min, max },
            step,
        }
    }

    /// Set the minimum bound of the range.
    pub fn with_min(self, val: T) -> Self {
        Self {
            range: self.range.with_min(val),
            ..self
        }
    }

    /// Set the maximum bound of the range.
    pub fn with_max(self, val: T) -> Self {
        Self {
            range: self.range.with_max(val),
            ..self
        }
    }

    /// Set the step value of the range.
    pub fn with_step(self, step: T) -> Self {
        Self {
            step: Some(step),
            ..self
        }
    }

    /// Get the minimum bound of the range, if it exists.
    pub fn min(&self) -> Option<T>
    where
        T: Copy,
    {
        self.range.min
    }

    /// Get the maximum bound of the range, if it exists.
    pub fn max(&self) -> Option<T>
    where
        T: Copy,
    {
        self.range.max
    }

    /// Get the step value of the range, if it exists.
    pub fn step(&self) -> Option<T>
    where
        T: Copy,
    {
        self.step
    }

    /// Clamps a value to be within the range defined by `min` and `max`, snapping it up to the nearest step if specified.
    pub fn fit(&self, value: T) -> T
    where
        T: PartialOrd + Copy + SnapUp,
    {
        let value = if let Some(step) = self.step {
            value.snap_up(step)
        } else {
            value
        };
        self.range.fit(value)
    }

    // Convert to inner range (useful for compatibility)
    pub fn into_range(self) -> PartialRange<T> {
        self.range
    }

    pub fn as_range(&self) -> &PartialRange<T> {
        &self.range
    }
}

impl<T> Default for SteppedRange<T> {
    fn default() -> Self {
        Self {
            range: PartialRange::default(),
            step: None,
        }
    }
}

// Delegate RangeBounds to the inner range
impl<T> std::ops::RangeBounds<T> for SteppedRange<T> {
    fn start_bound(&self) -> std::ops::Bound<&T> {
        self.range.start_bound()
    }

    fn end_bound(&self) -> std::ops::Bound<&T> {
        self.range.end_bound()
    }
}

// Easy conversion from PartialRange
impl<T> From<PartialRange<T>> for SteppedRange<T> {
    fn from(range: PartialRange<T>) -> Self {
        Self { range, step: None }
    }
}

impl<T> FromStr for SteppedRange<T>
where
    T: FromStr,
{
    type Err = SteppedRangeParseError<T::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Check for step syntax: "min..max:step"
        if let Some(colon_pos) = s.rfind(':') {
            let (range_part, step_part) = s.split_at(colon_pos);
            let step_part = &step_part[1..]; // Skip the ":"

            let range = range_part
                .parse::<PartialRange<T>>()
                .map_err(SteppedRangeParseError::RangeParseError)?;

            let step = if step_part.is_empty() {
                None
            } else {
                Some(
                    step_part
                        .parse::<T>()
                        .map_err(SteppedRangeParseError::StepParseError)?,
                )
            };

            Ok(SteppedRange { range, step })
        } else {
            // No step specified, parse as regular range
            let range = s
                .parse::<PartialRange<T>>()
                .map_err(SteppedRangeParseError::RangeParseError)?;
            Ok(SteppedRange { range, step: None })
        }
    }
}

impl<T> fmt::Display for SteppedRange<T>
where
    T: FromStr + Copy + fmt::Display,
    T::Err: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.range)?;
        if let Some(step) = self.step {
            write!(f, ":{}", step)?;
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum SteppedRangeParseError<E> {
    #[error(transparent)]
    RangeParseError(#[from] RangeParseError<E>),
    #[error("Failed to parse step value: {0}")]
    StepParseError(E),
}
