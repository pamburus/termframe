use super::*;
use itertools::Itertools;

#[test]
fn test_new_with_relevant_variants() {
    let wanted = "test";
    let variants = vec!["test", "testing", "toaster"];
    let suggestions = Suggestions::new(wanted, variants);

    assert_eq!(
        suggestions.candidates.iter().map(|x| &x.1).collect_vec(),
        ["test", "testing"]
    );
}

#[test]
fn test_new_with_no_relevant_variants() {
    let wanted = "test";
    let variants = vec!["apple", "banana", "carrot"];
    let suggestions = Suggestions::new(wanted, variants);

    assert!(suggestions.is_empty());
}

#[test]
fn test_none() {
    let suggestions = Suggestions::none();
    assert!(suggestions.is_empty());
}

#[test]
fn test_is_empty() {
    let suggestions = Suggestions::none();
    assert!(suggestions.is_empty());

    let wanted = "test";
    let variants = vec!["test"];
    let suggestions = Suggestions::new(wanted, variants);
    assert!(!suggestions.is_empty());
}

#[test]
fn test_iter() {
    let wanted = "test";
    let variants = vec!["test", "testing"];
    let suggestions = Suggestions::new(wanted, variants);

    let mut iter = suggestions.iter();
    assert_eq!(iter.next(), Some("test"));
    assert_eq!(iter.next(), Some("testing"));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_merge() {
    let wanted = "test";
    let variants1 = vec!["test"];
    let variants2 = vec!["testing"];
    let suggestions1 = Suggestions::new(wanted, variants1);
    let suggestions2 = Suggestions::new(wanted, variants2);

    let merged_suggestions = suggestions1.merge(suggestions2);
    assert_eq!(merged_suggestions.candidates.len(), 2);
    assert_eq!(merged_suggestions.candidates[0].1, "test");
    assert_eq!(merged_suggestions.candidates[1].1, "testing");
}
