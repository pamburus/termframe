// std imports
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let result = if args.len() >= 3 {
        let theme_dir = PathBuf::from(&args[1]);
        let theme_aliases = PathBuf::from(&args[2]);
        tidy_themes::update_theme_aliases(theme_dir, theme_aliases)
    } else {
        tidy_themes::update_theme_aliases_default()
    };

    if let Err(e) = result {
        eprintln!("\x1b[31mError:\x1b[0m {}", e);
        let mut source = e.source();
        while let Some(err) = source {
            eprintln!("\x1b[33mCaused by:\x1b[0m {}", err);
            source = err.source();
        }
        std::process::exit(1);
    }
}
