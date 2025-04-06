// std imports
use std::fmt;

// third-party imports
use clap::ValueEnum;
use serde::Deserialize;

/// Represents the mode of the application.
#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Dark,
    Light,
}

impl From<ModeSetting> for Mode {
    /// Converts a `ModeSetting` to a `Mode`.
    ///
    /// # Arguments
    ///
    /// * `mode_setting` - The `ModeSetting` to convert.
    ///
    /// # Returns
    ///
    /// A `Mode` corresponding to the given `ModeSetting`.
    fn from(mode_setting: ModeSetting) -> Self {
        match mode_setting {
            ModeSetting::Dark => Mode::Dark,
            ModeSetting::Light => Mode::Light,
            ModeSetting::Auto => match dark_light::detect() {
                Ok(dark_light::Mode::Dark) => {
                    log::info!("detected dark mode");
                    Mode::Dark
                }
                Ok(dark_light::Mode::Light) => {
                    log::info!("detected light mode");
                    Mode::Light
                }
                Ok(dark_light::Mode::Unspecified) => {
                    log::info!("dark or light mode is unspecified");
                    Mode::Dark
                }
                Err(e) => {
                    log::warn!("could not detect dark or light mode: {e}");
                    Mode::Dark
                }
            },
        }
    }
}

/// Represents the mode setting of the application.
#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ModeSetting {
    Auto,
    Dark,
    Light,
}

impl fmt::Display for ModeSetting {
    /// Formats the `ModeSetting` for display.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModeSetting::Auto => write!(f, "auto"),
            ModeSetting::Dark => write!(f, "dark"),
            ModeSetting::Light => write!(f, "light"),
        }
    }
}

impl TryFrom<&str> for ModeSetting {
    type Error = String;

    /// Attempts to convert a string slice to a `ModeSetting`.
    ///
    /// # Arguments
    ///
    /// * `value` - The string slice to convert.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `ModeSetting` if successful, or an error message if not.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "auto" => Ok(ModeSetting::Auto),
            "dark" => Ok(ModeSetting::Dark),
            "light" => Ok(ModeSetting::Light),
            _ => Err(format!("Invalid mode: {}", value)),
        }
    }
}
