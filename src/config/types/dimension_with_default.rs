use std::{fmt, str::FromStr};

use serde::Deserialize as DeriveDeserialize;
use serde::de::{Deserialize, Deserializer};

use super::{dimension::Dimension, range::PartialRange, snap::SnapUp, stepped_range::SteppedRange};

/// A dimension specification augmented with an optional default value.
///
/// This type wraps the existing `Dimension<T>` enum (which expresses either
/// an automatic value, a fixed value, or a stepped range constraint) and
/// associates it with an optional `default` value that can be used as an
/// initial/preferred value when not overridden by the CLI.
///
/// Key points:
/// - `default` is not a constraint; it is a preference/initial value.
/// - `dim` contains the constraints (or fixed/auto sentinel).
/// - Use `initial_or(fallback)` to obtain an initial value snapped/clamped
///   to the constraints, preferring `default` if present, then `Fixed`, else
///   falling back to the provided value.
///
/// Serde behavior (backward-compatible):
/// - Accepts all existing `Dimension<T>` syntaxes (string/number/range/table without `default`)
///   and maps them to `DimensionWithDefault { dim, default: None }`.
/// - Also accepts a table form with optional `{ min, max, step, default }`.
///   When only `default` is specified (no min/max/step), `dim` becomes `Auto` and
///   `default` is preserved.
///   Otherwise, `dim` is constructed as `Limited(SteppedRange { min, max, step })`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionWithDefault<T> {
    pub dim: Dimension<T>,
    pub default: Option<T>,
}

#[derive(Debug, DeriveDeserialize)]
#[serde(untagged)]
enum DimensionWithDefaultRaw<T> {
    // Accept table with optional default, and optional constraints
    Spec {
        #[serde(default)]
        min: Option<T>,
        #[serde(default)]
        max: Option<T>,
        #[serde(default)]
        step: Option<T>,
        #[serde(default)]
        default: Option<T>,
    },
    // Accept any existing Dimension<T> representation (string/number/range/table)
    Simple(Dimension<T>),
}

impl<'de, T> Deserialize<'de> for DimensionWithDefault<T>
where
    T: Copy + DeriveDeserialize<'de> + Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = DimensionWithDefaultRaw::<T>::deserialize(deserializer)?;
        Ok(match raw {
            DimensionWithDefaultRaw::Simple(dim) => Self { dim, default: None },
            DimensionWithDefaultRaw::Spec {
                min,
                max,
                step,
                default,
            } => {
                // If no constraints provided, treat as Auto with a default
                if min.is_none() && max.is_none() && step.is_none() {
                    Self {
                        dim: Dimension::Auto,
                        default,
                    }
                } else {
                    let range = SteppedRange {
                        range: PartialRange { min, max },
                        step,
                    };
                    Self {
                        dim: Dimension::Limited(range),
                        default,
                    }
                }
            }
        })
    }
}

impl<T> From<Dimension<T>> for DimensionWithDefault<T>
where
    T: Copy,
{
    fn from(dim: Dimension<T>) -> Self {
        Self { dim, default: None }
    }
}

impl<T> DimensionWithDefault<T>
where
    T: Copy,
{
    /// Returns the minimum bound if present.
    pub fn min(&self) -> Option<T> {
        self.dim.min()
    }

    /// Returns the maximum bound if present.
    pub fn max(&self) -> Option<T> {
        self.dim.max()
    }

    /// Returns the step if present.
    pub fn step(&self) -> Option<T> {
        self.dim.step()
    }

    /// Returns a `SteppedRange` representation of the dimension (fixed/auto normalized).
    pub fn range(&self) -> SteppedRange<T> {
        self.dim.range()
    }
}

impl<T> DimensionWithDefault<T>
where
    T: PartialOrd + Copy + SnapUp,
{
    /// Clamps and snaps `value` to this dimension's constraints.
    pub fn fit(&self, value: T) -> T {
        self.dim.fit(value)
    }

    /// Resolve an initial value:
    /// - If `default` is present, return `fit(default)`.
    /// - Else, if `dim` is `Fixed(v)`, return `v`.
    /// - Else, return `fit(fallback)`.
    pub fn initial_or(&self, fallback: T) -> T {
        if let Some(d) = self.default {
            return self.fit(d);
        }
        match self.dim {
            Dimension::Fixed(v) => v,
            _ => self.fit(fallback),
        }
    }
}

impl<T> fmt::Display for DimensionWithDefault<T>
where
    T: FromStr + Copy + fmt::Display,
    T::Err: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.default {
            Some(d) => write!(f, "{}@{d}", self.dim),
            None => write!(f, "{}", self.dim),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use toml;

    #[derive(Deserialize)]
    struct ConfigSimple {
        dim: DimensionWithDefault<u16>,
    }

    #[test]
    fn test_deserialize_simple_auto() {
        let cfg: ConfigSimple = toml::from_str(r#"dim = "auto""#).unwrap();
        assert_eq!(cfg.dim.dim, Dimension::Auto);
        assert_eq!(cfg.dim.default, None);
    }

    #[test]
    fn test_deserialize_simple_fixed() {
        let cfg: ConfigSimple = toml::from_str("dim = 100").unwrap();
        assert_eq!(cfg.dim.dim, Dimension::Fixed(100));
        assert_eq!(cfg.dim.default, None);
    }

    #[test]
    fn test_deserialize_simple_range_table() {
        // Existing Dimension table (no default) remains valid
        let cfg: ConfigSimple =
            toml::from_str(r#"dim = { min = 80, max = 120, step = 4 }"#).unwrap();
        match cfg.dim.dim {
            Dimension::Limited(sr) => {
                assert_eq!(sr.range.min, Some(80));
                assert_eq!(sr.range.max, Some(120));
                assert_eq!(sr.step, Some(4));
            }
            _ => panic!("expected Limited"),
        }
        assert_eq!(cfg.dim.default, None);
    }

    #[test]
    fn test_deserialize_with_default_only() {
        // Only default: Auto constraints with a preferred starting value
        let cfg: ConfigSimple = toml::from_str(r#"dim = { default = 160 }"#).unwrap();
        assert_eq!(cfg.dim.dim, Dimension::Auto);
        assert_eq!(cfg.dim.default, Some(160));
    }

    #[test]
    fn test_deserialize_with_range_and_default() {
        let cfg: ConfigSimple =
            toml::from_str(r#"dim = { min = 80, max = 240, step = 4, default = 160 }"#).unwrap();
        match cfg.dim.dim {
            Dimension::Limited(sr) => {
                assert_eq!(sr.range.min, Some(80));
                assert_eq!(sr.range.max, Some(240));
                assert_eq!(sr.step, Some(4));
            }
            _ => panic!("expected Limited"),
        }
        assert_eq!(cfg.dim.default, Some(160));
    }

    #[test]
    fn test_initial_or_snaps_and_clamps() {
        // Range [80..100] step=5, default=99 => snaps to 100
        let cfg: ConfigSimple =
            toml::from_str(r#"dim = { min = 80, max = 100, step = 5, default = 99 }"#).unwrap();
        let init = cfg.dim.initial_or(0);
        assert_eq!(init, 100);

        // No default, fixed=90 => initial is fixed 90
        let cfg: ConfigSimple = toml::from_str(r#"dim = 90"#).unwrap();
        let init = cfg.dim.initial_or(0);
        assert_eq!(init, 90);

        // Auto with default clamps to fallback fit
        let cfg: ConfigSimple = toml::from_str(r#"dim = "auto""#).unwrap();
        let init = cfg.dim.initial_or(42);
        assert_eq!(init, 42);
    }
}
