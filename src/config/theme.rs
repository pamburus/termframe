// std imports
use std::{collections::HashMap, sync::LazyLock};

// third-party imports
use csscolorparser::Color;
use rust_embed::RustEmbed;
use serde::Deserialize;

// local imports
use super::{load::Load, mode::Mode};

// ---

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum ThemeConfig {
    Fixed(Fixed),
    Adaptive(Adaptive),
}

impl ThemeConfig {
    pub fn resolve(&self, mode: Mode) -> &Colors {
        match self {
            ThemeConfig::Fixed(fixed) => &fixed.colors,
            ThemeConfig::Adaptive(dynamic) => match mode {
                Mode::Dark => &dynamic.modes.dark,
                Mode::Light => &dynamic.modes.light,
            },
        }
    }
}

impl Load for ThemeConfig {
    type Assets = Assets;

    fn dir_name() -> &'static str {
        "themes"
    }

    fn resolve_embedded_name_alias(alias: &str) -> &str {
        ALIAS_MAP.name(alias).unwrap_or(alias)
    }

    fn preferred_embedded_name_alias(name: &str) -> &str {
        ALIAS_MAP.alias(name).unwrap_or(name)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Fixed {
    pub colors: Colors,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Adaptive {
    pub modes: Modes,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Modes {
    pub dark: Colors,
    pub light: Colors,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Colors {
    pub background: Color,
    pub foreground: Color,
    pub palette: Palette,
}

pub type Palette = HashMap<u8, Color>;

// ---

struct AliasMap {
    a2n: HashMap<String, String>,
    n2a: HashMap<String, String>,
}

impl AliasMap {
    fn load() -> Self {
        let asset = Assets::get(".aliases.json").unwrap();
        Self::new(serde_json::from_slice(&asset.data).unwrap())
    }

    fn new(a2n: HashMap<String, String>) -> Self {
        let mut n2a = HashMap::new();
        for (alias, name) in a2n.iter() {
            n2a.insert(name.clone(), alias.clone());
        }

        Self { a2n, n2a }
    }

    fn alias(&self, name: &str) -> Option<&str> {
        self.n2a.get(name).map(|s| s.as_str())
    }

    fn name(&self, alias: &str) -> Option<&str> {
        self.a2n.get(alias).map(|s| s.as_str())
    }
}

// ---

#[derive(RustEmbed)]
#[folder = "src/assets/themes/"]
pub struct Assets;

static ALIAS_MAP: LazyLock<AliasMap> = LazyLock::new(|| AliasMap::load());
