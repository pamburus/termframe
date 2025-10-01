fn main() {
    if let Err(e) = tidy_themes::update_theme_aliases_default() {
        panic!("Failed to update theme aliases: {}", e);
    }

    println!("cargo:rerun-if-changed=assets/themes");
}
