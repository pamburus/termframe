// std imports
use std::collections::BTreeMap;

// third-party imports
use voca_rs::case::kebab_case;

const THEME_EXTENSION: &str = ".yaml";
const THEME_DIR: &str = "assets/themes";
const THEME_ALIASES: &str = "assets/themes/.aliases.json";

fn main() {
    update_theme_aliases();
}

fn update_theme_aliases() {
    let mut aliases = BTreeMap::<String, String>::new();

    println!("cargo:rerun-if-changed={THEME_DIR}");
    for entry in std::fs::read_dir(THEME_DIR).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if path.is_file() && name.ends_with(THEME_EXTENSION) {
            let name = name.trim_end_matches(THEME_EXTENSION);
            let alias = kebab_case(&name.replace("+", " plus "));
            if alias != name {
                if let Some(other) = aliases.insert(alias.to_owned(), name.to_owned()) {
                    panic!("Conflicting aliases for {} and {}: {}", other, name, alias);
                }
            }
        }
    }

    if let Ok(existing) = std::fs::read(THEME_ALIASES) {
        let existing: BTreeMap<String, String> = serde_json::from_slice(&existing).unwrap();
        if existing == aliases {
            return;
        }
    }

    std::fs::write(
        THEME_ALIASES,
        serde_json::to_string_pretty(&aliases).unwrap(),
    )
    .unwrap();
}
