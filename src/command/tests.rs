use super::*;

fn to_terminal_str(prompt: &str, command: &str, args: &[&str], theme: Option<Theme>) -> String {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    String::from_utf8(to_terminal(prompt, command, &args, theme)).unwrap()
}

fn test_theme() -> Theme {
    "dracula".parse().unwrap()
}

#[test]
fn test_to_title() {
    let title = to_title(Some("git"), vec!["status", "-s"]);

    assert!(title.is_some());
    assert_eq!(title.unwrap(), "git status -s");
}

#[test]
fn test_to_title_with_special_chars() {
    let title = to_title(Some("echo"), vec!["Hello, World!", "\"quoted\"", "$HOME"]);

    let title_str = title.unwrap();
    assert!(title_str.contains("echo"));
    assert!(title_str.contains("Hello,") && title_str.contains("World"));
    assert!(title_str.contains("quoted"));
    assert!(title_str.contains("HOME"));
}

#[test]
fn test_to_terminal_with_theme() {
    let s = to_terminal_str("$ ", "echo", &["hello"], Some(test_theme()));

    assert!(s.starts_with("\x1b[35m$ \x1b[0m"));
    assert!(s.contains("echo"));
    assert!(s.contains("hello"));
    assert!(s.ends_with('\n'));
    assert!(s.contains("\x1b["));
}

#[test]
fn test_to_terminal_without_theme() {
    let s = to_terminal_str("$ ", "echo", &["hello"], None);

    assert!(s.starts_with("\x1b[35m$ \x1b[0m"));
    assert!(s.contains("echo"));
    assert!(s.contains("hello"));
    assert!(s.ends_with('\n'));
}

#[test]
fn test_to_terminal_empty_prompt() {
    let s = to_terminal_str("", "ls", &["-la"], Some(test_theme()));

    assert!(s.starts_with("\x1b[35m\x1b[0m"));
    assert!(s.contains("ls"));
    assert!(s.contains("-la"));
    assert!(s.ends_with('\n'));
}

#[test]
fn test_to_terminal_custom_prompt() {
    let s = to_terminal_str("% ", "git", &["status"], Some(test_theme()));

    assert!(s.starts_with("\x1b[35m% \x1b[0m"));
    assert!(s.contains("git"));
    assert!(s.contains("status"));
}

#[test]
fn test_to_terminal_special_chars() {
    let s = to_terminal_str("$ ", "echo", &["Hello, World!"], Some(test_theme()));

    assert!(s.starts_with("\x1b[35m$ \x1b[0m"));
    assert!(s.contains("echo"));
    assert!(s.contains("Hello,"));
    assert!(s.contains("World"));
}
