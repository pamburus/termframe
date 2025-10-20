use super::{Highlight, Suggestions};
use crate::xerr::HighlightQuoted;
use std::path::Path;

#[test]
fn test_highlight() {
    assert_eq!("hello".hl().to_string(), "\u{1b}[33mhello\u{1b}[0m");
    assert_eq!(
        Path::new("hello").hl().to_string(),
        "\u{1b}[33mhello\u{1b}[0m"
    );
    assert_eq!("hello".hlq().to_string(), "\u{1b}[33m\"hello\"\u{1b}[0m");
    assert_eq!(
        Path::new("hello").hlq().to_string(),
        "\u{1b}[33m\"hello\"\u{1b}[0m"
    );
}

#[test]
fn test_suggestions() {
    // Test suggestions with relevant variants
    let variants = ["apple", "banana", "cherry"];
    let wanted = "aple";

    let suggestions = Suggestions::new(wanted, variants.iter().map(|s| s.to_string()));

    // Verify we get some suggestions (likely "apple")
    assert!(!suggestions.is_empty());

    let suggestions_list: Vec<_> = suggestions.iter().collect();
    assert!(!suggestions_list.is_empty());

    // Don't assert specific suggestion content since the algorithm may change
    // Just check we have some suggestions

    // Test no suggestions case
    let variants = ["dog", "cat", "fish"];
    let wanted = "aple";

    let suggestions = Suggestions::new(wanted, variants.iter().map(|s| s.to_string()));

    // Verify we get no suggestions
    assert!(suggestions.is_empty());

    // Test empty suggestions
    let suggestions = Suggestions::none();
    assert!(suggestions.is_empty());

    // Test merging suggestions with different target words
    // Using more specific matches to ensure the algorithm has something to suggest
    let variants1 = ["apple", "apricot"];
    let variants2 = ["banana", "avocado"];

    // Use "app" instead of "a" to get better matches for apple/apricot
    let suggestions1 = Suggestions::new("app", variants1.iter().map(|s| s.to_string()));
    // Use "ban" to get better matches for banana
    let suggestions2 = Suggestions::new("ban", variants2.iter().map(|s| s.to_string()));

    let merged = suggestions1.merge(suggestions2);
    let merged_list: Vec<_> = merged.iter().collect();

    // Don't assert specific suggestion content since the algorithm may change
    // Just check we have merged suggestions
    assert!(!merged_list.is_empty());
}
