// std imports
use std::collections::BTreeMap;
use std::path::Path;

// third-party imports
use anyhow::{Context, Result};
use voca_rs::case::kebab_case;

const THEME_EXTENSION: &str = ".toml";

pub fn update_theme_aliases<P: AsRef<Path>>(theme_dir: P, theme_aliases: P) -> Result<()> {
    let mut aliases = BTreeMap::<String, String>::new();

    for entry in std::fs::read_dir(&theme_dir).with_context(|| {
        format!(
            "Failed to read theme directory {}",
            theme_dir.as_ref().display()
        )
    })? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if path.is_file() && name.ends_with(THEME_EXTENSION) {
            let name = name.trim_end_matches(THEME_EXTENSION);
            let alias = kebab_case(&name.replace("+", " plus "));
            if alias != name
                && let Some(other) = aliases.insert(alias.to_owned(), name.to_owned())
            {
                return Err(anyhow::anyhow!(
                    "Conflicting aliases for {other} and {name}: {alias}"
                ));
            }
        }
    }

    if let Ok(existing) = std::fs::read(&theme_aliases) {
        let existing: BTreeMap<String, String> =
            serde_json::from_slice(&existing).with_context(|| {
                format!(
                    "Failed to parse existing aliases file {:?}",
                    theme_aliases.as_ref().display()
                )
            })?;
        if existing == aliases {
            return Ok(());
        }
    }

    let json_str = serde_json::to_string_pretty(&aliases).context("Failed to serialize aliases")?;

    std::fs::write(&theme_aliases, json_str).with_context(|| {
        format!(
            "Failed to write aliases file {:?}",
            theme_aliases.as_ref().display()
        )
    })?;

    Ok(())
}

pub fn update_theme_aliases_default() -> Result<()> {
    update_theme_aliases("assets/themes", "assets/themes/.aliases.json")
}
