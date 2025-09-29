// std imports
use std::{collections::BTreeMap, env, fs, path::PathBuf, sync::LazyLock};

// third-party imports
use voca_rs::case::kebab_case;

const THEME_EXTENSION: &str = ".toml";
const THEME_DIR: &str = "assets/themes";
const THEME_ALIASES: &str = "assets/themes/.aliases.json";

static ROOT: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var("ROOT").unwrap_or_else(|_| ".".to_string())));

fn main() {
    update_theme_aliases();
}

fn update_theme_aliases() {
    let mut aliases = BTreeMap::<String, String>::new();
    let theme_dir = ROOT.join(THEME_DIR);
    let theme_aliases = ROOT.join(THEME_ALIASES);

    println!("cargo:rerun-if-changed={}", theme_dir.display());
    for entry in fs::read_dir(&theme_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if path.is_file() && name.ends_with(THEME_EXTENSION) {
            let name = name.trim_end_matches(THEME_EXTENSION);
            let alias = kebab_case(&name.replace("+", " plus "));
            if alias != name
                && let Some(other) = aliases.insert(alias.to_owned(), name.to_owned())
            {
                panic!("Conflicting aliases for {other} and {name}: {alias}");
            }
        }
    }

    if let Ok(existing) = fs::read(&theme_aliases) {
        let existing: BTreeMap<String, String> = serde_json::from_slice(&existing).unwrap();
        if existing == aliases {
            return;
        }
    }

    fs::write(
        &theme_aliases,
        serde_json::to_string_pretty(&aliases).unwrap(),
    )
    .unwrap();
}
