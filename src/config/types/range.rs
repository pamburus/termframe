// std imports
use std::{fmt, str::FromStr};

// third-party imports
use serde::Deserialize;
use thiserror::Error;

/// Range type for parsing range syntax like "80..", "..120", "80..120" from strings or deserializing from structs.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Deserialize)]
pub struct PartialRange<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T> PartialRange<T> {
    /// Create a new `PartialRange` with the given minimum and maximum bounds.
    pub fn new(min: Option<T>, max: Option<T>) -> Self {
        Self { min, max }
    }

    /// Set the minimum bound of the range.
    pub fn with_min(self, val: T) -> Self {
        Self {
            min: Some(val),
            ..self
        }
    }

    /// Set the maximum bound of the range.
    pub fn with_max(self, val: T) -> Self {
        Self {
            max: Some(val),
            ..self
        }
    }

    /// Get the minumum bound of the range, if it exists.
    pub fn min(&self) -> Option<T>
    where
        T: Copy,
    {
        self.min
    }

    /// Get the maxumum bound of the range, if it exists.
    pub fn max(&self) -> Option<T>
    where
        T: Copy,
    {
        self.max
    }

    /// Clamps a value to be within the range defined by `min` and `max`.
    pub fn clamp(&self, value: T) -> T
    where
        T: PartialOrd + Copy,
    {
        let value = match self.min {
            Some(min) if value < min => min,
            _ => value,
        };
        match self.max {
            Some(max) if value > max => max,
            _ => value,
        }
    }
}

impl<T> Default for PartialRange<T> {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
        }
    }
}

impl<T> std::ops::RangeBounds<T> for PartialRange<T> {
    fn start_bound(&self) -> std::ops::Bound<&T> {
        match &self.min {
            Some(min) => std::ops::Bound::Included(min),
            None => std::ops::Bound::Unbounded,
        }
    }

    fn end_bound(&self) -> std::ops::Bound<&T> {
        match &self.max {
            Some(max) => std::ops::Bound::Included(max),
            None => std::ops::Bound::Unbounded,
        }
    }
}

impl<T> From<(T, T)> for PartialRange<T> {
    fn from((min, max): (T, T)) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }
}

impl<T> FromStr for PartialRange<T>
where
    T: FromStr,
{
    type Err = RangeParseError<T::Err>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(dot_pos) = s.find("..") else {
            return Err(RangeParseError::MissingDots);
        };

        if s[dot_pos + 2..].contains("..") {
            return Err(RangeParseError::InvalidFormat);
        }

        let (min_str, max_str) = s.split_at(dot_pos);
        let max_str = &max_str[2..]; // Skip the ".."

        let min = if min_str.is_empty() {
            None
        } else {
            Some(
                min_str
                    .parse::<T>()
                    .map_err(RangeParseError::BoundParseError)?,
            )
        };

        let max = if max_str.is_empty() {
            None
        } else {
            Some(
                max_str
                    .parse::<T>()
                    .map_err(RangeParseError::BoundParseError)?,
            )
        };

        Ok(PartialRange { min, max })
    }
}

impl<T> fmt::Display for PartialRange<T>
where
    T: FromStr + Copy + fmt::Display,
    T::Err: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.min, self.max) {
            (Some(min), Some(max)) => write!(f, "{min}..{max}"),
            (Some(min), None) => write!(f, "{min}.."),
            (None, Some(max)) => write!(f, "..{max}"),
            (None, None) => write!(f, ".."),
        }
    }
}

#[derive(Error, Debug)]
pub enum RangeParseError<E> {
    MissingDots,
    InvalidFormat,
    BoundParseError(E),
}

impl<E> std::fmt::Display for RangeParseError<E>
where
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangeParseError::MissingDots => write!(f, "expected range syntax with '..'"),
            RangeParseError::InvalidFormat => write!(f, "invalid range format"),
            RangeParseError::BoundParseError(e) => write!(f, "bound parse error: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_partial_range_from_str() {
        use super::*;

        let range: PartialRange<u16> = "80..120".parse().unwrap();
        assert_eq!(range.min, Some(80));
        assert_eq!(range.max, Some(120));

        let range: PartialRange<u16> = "80..".parse().unwrap();
        assert_eq!(range.min, Some(80));
        assert_eq!(range.max, None);

        let range: PartialRange<u16> = "..120".parse().unwrap();
        assert_eq!(range.min, None);
        assert_eq!(range.max, Some(120));

        let range: Result<PartialRange<u16>, _> = "80".parse();
        assert!(range.is_err());

        let range: Result<PartialRange<u16>, _> = "80...120".parse();
        assert!(range.is_err());
    }

    #[test]
    fn test_partial_range_deserialize() {
        use super::*;
        use toml;

        #[derive(Deserialize, Debug)]
        struct TestStruct {
            range: PartialRange<u32>,
        }

        let toml_str = r#"range = { min = 80, max = 120 }"#;
        let result: TestStruct = toml::from_str(toml_str).unwrap();
        assert_eq!(result.range.min, Some(80));
        assert_eq!(result.range.max, Some(120));

        let toml_str = r#"range = { min = 80 }"#;
        let result: TestStruct = toml::from_str(toml_str).unwrap();
        assert_eq!(result.range.min, Some(80));
        assert_eq!(result.range.max, None);

        let toml_str = r#"range = { max = 120 }"#;
        let result: TestStruct = toml::from_str(toml_str).unwrap();
        assert_eq!(result.range.min, None);
        assert_eq!(result.range.max, Some(120));

        let toml_str = r#"range = {}"#;
        let result: TestStruct = toml::from_str(toml_str).unwrap();
        assert_eq!(result.range.min, None);
        assert_eq!(result.range.max, None);
    }
}
