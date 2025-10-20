use std::{fmt, str::FromStr};

use serde::Deserialize as DeriveDeserialize;
use serde::de::{Deserialize, Deserializer};

use super::{dimension::Dimension, range::PartialRange, snap::SnapUp, stepped_range::SteppedRange};

/// A dimension specification augmented with an optional default value.
///
/// This type wraps the existing `Dimension<T>` enum (which expresses either
/// an automatic value, a fixed value, or a stepped range constraint) and
/// associates it with an optional `initial` value that can be used as an
/// initial/preferred value when not overridden by the CLI.
///
/// Key points:
/// - `initial` is not a constraint; it is a preference/initial value.
/// - `dim` contains the constraints (or fixed/auto sentinel).
/// - Use `initial_or(fallback)` to obtain an initial value snapped/clamped
///   to the constraints, preferring `initial` if present, then `Fixed`, else
///   falling back to the provided value.
///
/// Serde behavior (backward-compatible):
/// - Accepts all existing `Dimension<T>` syntaxes (string/number/range/table without `initial`)
///   and maps them to `DimensionWithInitial { dim, initial: None }`.
/// - Also accepts a table form with optional `{ min, max, step, initial }`.
///   When only `initial` is specified (no min/max/step), `dim` becomes `Auto` and
///   `initial` is preserved.
///   Otherwise, `dim` is constructed as `Limited(SteppedRange { min, max, step })`.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct DimensionWithInitial<T> {
    pub current: Dimension<T>,
    pub initial: Option<T>,
}

impl<T> DimensionWithInitial<T>
where
    T: Copy,
{
    /// Returns the minimum bound if present.
    pub fn min(&self) -> Option<T> {
        self.current.min()
    }

    /// Returns the maximum bound if present.
    pub fn max(&self) -> Option<T> {
        self.current.max()
    }

    /// Returns the step if present.
    pub fn step(&self) -> Option<T> {
        self.current.step()
    }

    /// Returns a `SteppedRange` representation of the dimension (fixed/auto normalized).
    pub fn range(&self) -> SteppedRange<T> {
        self.current.range()
    }
}

impl<T> DimensionWithInitial<T>
where
    T: PartialOrd + Copy + SnapUp,
{
    /// Clamps and snaps `value` to this dimension's constraints.
    pub fn fit(&self, value: T) -> T {
        self.current.fit(value)
    }

    /// Resolve an initial value:
    /// - If `initial` is present, return `fit(initial)`.
    /// - Else, if `dim` is `Fixed(v)`, return `v`.
    /// - Else, return `fit(fallback)`.
    pub fn initial_or(&self, fallback: T) -> T {
        if let Some(d) = self.initial {
            return self.fit(d);
        }
        match self.current {
            Dimension::Fixed(v) => v,
            _ => self.fit(fallback),
        }
    }
}

impl<T> From<DimensionWithInitial<T>> for Dimension<T> {
    fn from(v: DimensionWithInitial<T>) -> Self {
        v.current
    }
}

impl<'de, T> Deserialize<'de> for DimensionWithInitial<T>
where
    T: Copy + DeriveDeserialize<'de> + Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = DimensionWithInitialRaw::<T>::deserialize(deserializer)?;
        Ok(match raw {
            DimensionWithInitialRaw::Simple(dim) => Self {
                current: dim,
                initial: None,
            },
            DimensionWithInitialRaw::Spec {
                min,
                max,
                step,
                initial,
            } => {
                // If no constraints provided, treat as Auto with a default
                if min.is_none() && max.is_none() && step.is_none() {
                    Self {
                        current: Dimension::Auto,
                        initial,
                    }
                } else {
                    let range = SteppedRange {
                        range: PartialRange { min, max },
                        step,
                    };
                    Self {
                        current: Dimension::Limited(range),
                        initial,
                    }
                }
            }
        })
    }
}

impl<T> From<Dimension<T>> for DimensionWithInitial<T>
where
    T: Copy,
{
    fn from(dim: Dimension<T>) -> Self {
        Self {
            current: dim,
            initial: None,
        }
    }
}

impl<T> From<T> for DimensionWithInitial<T>
where
    T: Copy,
{
    fn from(value: T) -> Self {
        Self {
            current: Dimension::Fixed(value),
            initial: None,
        }
    }
}

impl<T> fmt::Display for DimensionWithInitial<T>
where
    T: FromStr + Copy + fmt::Display,
    T::Err: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.initial {
            Some(d) => write!(f, "{}@{d}", self.current),
            None => write!(f, "{}", self.current),
        }
    }
}

impl<T> FromStr for DimensionWithInitial<T>
where
    T: FromStr + Copy,
    T::Err: fmt::Display,
{
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Support suffix "@initial" syntax like "80..240:4@160"
        if let Some(at) = s.rfind('@') {
            let (left, right) = s.split_at(at);
            let def_str = &right[1..];

            // Parse the dimension (left side). Empty left means "auto".
            let dim = if left.trim().is_empty() {
                Dimension::Auto
            } else {
                Dimension::from_str(left).map_err(|e| e.to_string())?
            };

            // Parse the initial (right side) if present (non-empty)
            let initial = if def_str.trim().is_empty() {
                None
            } else {
                Some(def_str.parse::<T>().map_err(|e| e.to_string())?)
            };

            Ok(Self {
                current: dim,
                initial,
            })
        } else {
            // Fallback: parse as a plain Dimension<T> (no initial provided)
            let dim = Dimension::from_str(s).map_err(|e| e.to_string())?;
            Ok(Self {
                current: dim,
                initial: None,
            })
        }
    }
}

#[derive(Debug, DeriveDeserialize)]
#[serde(untagged)]
enum DimensionWithInitialRaw<T> {
    // Accept table with optional initial, and optional constraints
    Spec {
        #[serde(default)]
        min: Option<T>,
        #[serde(default)]
        max: Option<T>,
        #[serde(default)]
        step: Option<T>,
        #[serde(default)]
        initial: Option<T>,
    },
    // Accept any existing Dimension<T> representation (string/number/range/table)
    Simple(Dimension<T>),
}

#[cfg(test)]
mod tests;
