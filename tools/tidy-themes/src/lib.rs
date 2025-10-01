// std imports
use std::collections::BTreeMap;
use std::path::Path;

// third-party imports
use voca_rs::case::kebab_case;

const THEME_EXTENSION: &str = ".toml";

pub fn update_theme_aliases<P: AsRef<Path>>(
    theme_dir: P,
    theme_aliases: P,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut aliases = BTreeMap::<String, String>::new();

    for entry in std::fs::read_dir(&theme_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if path.is_file() && name.ends_with(THEME_EXTENSION) {
            let name = name.trim_end_matches(THEME_EXTENSION);
            let alias = kebab_case(&name.replace("+", " plus "));
            if alias != name
                && let Some(other) = aliases.insert(alias.to_owned(), name.to_owned())
            {
                return Err(format!("Conflicting aliases for {other} and {name}: {alias}").into());
            }
        }
    }

    if let Ok(existing) = std::fs::read(&theme_aliases) {
        let existing: BTreeMap<String, String> = serde_json::from_slice(&existing)?;
        if existing == aliases {
            return Ok(());
        }
    }

    std::fs::write(&theme_aliases, serde_json::to_string_pretty(&aliases)?)?;

    Ok(())
}

pub fn update_theme_aliases_default() -> Result<(), Box<dyn std::error::Error>> {
    update_theme_aliases("assets/themes", "assets/themes/.aliases.json")
}
