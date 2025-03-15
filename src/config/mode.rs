use serde::Deserialize;

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Dark,
    Light,
}

impl From<ModeSetting> for Mode {
    fn from(mode_setting: ModeSetting) -> Self {
        match mode_setting {
            ModeSetting::Dark => Mode::Dark,
            ModeSetting::Light => Mode::Light,
            ModeSetting::Auto => match dark_light::detect().ok() {
                Some(dark_light::Mode::Dark) => {
                    log::info!("detected dark mode");
                    Mode::Dark
                }
                Some(dark_light::Mode::Light) => {
                    log::info!("detected light mode");
                    Mode::Light
                }
                _ => {
                    log::info!("could not detect dark or light mode, fallback to dark");
                    Mode::Dark
                }
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ModeSetting {
    Auto,
    Dark,
    Light,
}

impl ToString for ModeSetting {
    fn to_string(&self) -> String {
        match self {
            ModeSetting::Auto => "auto".to_string(),
            ModeSetting::Dark => "dark".to_string(),
            ModeSetting::Light => "light".to_string(),
        }
    }
}

impl TryFrom<&str> for ModeSetting {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "auto" => Ok(ModeSetting::Auto),
            "dark" => Ok(ModeSetting::Dark),
            "light" => Ok(ModeSetting::Light),
            _ => Err(format!("Invalid mode: {}", value)),
        }
    }
}
