// third-party imports
use tempfile::tempdir;

// local imports
use super::*;

#[test]
fn sanitize_file_name_replaces_reserved_chars() {
    assert_eq!(
        sanitize_file_name("Paper Crane: Midnight Ink.toml"),
        "Paper Crane - Midnight Ink.toml"
    );
    assert_eq!(sanitize_file_name("What?.toml"), "What-.toml");
    assert_eq!(sanitize_file_name("One Double.toml"), "One Double.toml");
}

#[test]
fn sanitized_name_keeps_alias() {
    assert_eq!(
        kebab_case("Paper Crane: Midnight Ink"),
        kebab_case(&sanitize_file_name("Paper Crane: Midnight Ink"))
    );
}

// Windows cannot create files with reserved characters in their names.
#[cfg(unix)]
#[test]
fn sanitize_theme_file_names_renames_files() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("Some: Theme.toml"), "content").unwrap();
    std::fs::write(dir.path().join("Other.toml"), "other").unwrap();

    sanitize_theme_file_names(dir.path()).unwrap();

    assert!(!dir.path().join("Some: Theme.toml").exists());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("Some - Theme.toml")).unwrap(),
        "content"
    );
    assert!(dir.path().join("Other.toml").exists());
}

// Windows cannot create files with reserved characters in their names.
#[cfg(unix)]
#[test]
fn sanitize_theme_file_names_rejects_conflicts() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("Some: Theme.toml"), "content").unwrap();
    std::fs::write(dir.path().join("Some - Theme.toml"), "other").unwrap();

    let err = sanitize_theme_file_names(dir.path()).unwrap_err();

    assert!(err.to_string().contains("Conflicting theme file names"));
}

#[test]
fn sanitize_theme_file_names_fails_on_missing_directory() {
    let dir = tempdir().unwrap();

    let err = sanitize_theme_file_names(dir.path().join("missing")).unwrap_err();

    assert!(err.to_string().contains("Failed to read theme directory"));
}
