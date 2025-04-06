use std::cmp::Ordering;

const MIN_RELEVANCE: f64 = 0.75;

/// A struct to hold suggestions with their relevance scores.
#[derive(Debug, Clone)]
pub struct Suggestions {
    candidates: Vec<(f64, String)>,
}

impl Suggestions {
    /// Creates a new `Suggestions` instance by comparing the `wanted` string with a list of `variants`.
    ///
    /// # Arguments
    ///
    /// * `wanted` - The string to compare against.
    /// * `variants` - An iterator over the variants to compare.
    ///
    /// # Returns
    ///
    /// A `Suggestions` instance containing the relevant suggestions.
    pub fn new<T, I>(wanted: &str, variants: I) -> Self
    where
        T: AsRef<str>,
        I: IntoIterator<Item = T>,
    {
        let mut candidates = Vec::<(f64, String)>::new();

        for variant in variants {
            let relevance = strsim::jaro(wanted, variant.as_ref());

            if relevance > MIN_RELEVANCE {
                let candidate = (relevance, variant.as_ref().to_owned());
                let pos = candidates
                    .binary_search_by(|candidate| {
                        if candidate.0 < relevance {
                            Ordering::Greater
                        } else {
                            Ordering::Less
                        }
                    })
                    .unwrap_or_else(|e| e);
                candidates.insert(pos, candidate);
            }
        }

        Self { candidates }
    }

    /// Creates an empty `Suggestions` instance.
    ///
    /// # Returns
    ///
    /// An empty `Suggestions` instance.
    pub fn none() -> Self {
        Self {
            candidates: Vec::new(),
        }
    }

    /// Checks if there are no suggestions.
    ///
    /// # Returns
    ///
    /// `true` if there are no suggestions, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Returns an iterator over the suggestions.
    ///
    /// # Returns
    ///
    /// An iterator over the suggestions.
    pub fn iter(&self) -> SuggestionsIter {
        SuggestionsIter {
            iter: self.candidates.iter(),
        }
    }

    /// Merges another `Suggestions` instance into this one.
    ///
    /// # Arguments
    ///
    /// * `other` - Another `Suggestions` instance to merge.
    ///
    /// # Returns
    ///
    /// A new `Suggestions` instance containing the merged suggestions.
    pub fn merge(self, other: Self) -> Self {
        let mut candidates = self.candidates;

        for (relevance, candidate) in other.candidates {
            let pos = candidates
                .binary_search_by(|c| {
                    if c.0 < relevance {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                })
                .unwrap_or_else(|e| e);
            candidates.insert(pos, (relevance, candidate));
        }

        Self { candidates }
    }
}

impl<'a> IntoIterator for &'a Suggestions {
    type Item = &'a str;
    type IntoIter = SuggestionsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SuggestionsIter {
            iter: self.candidates.iter(),
        }
    }
}

/// An iterator over the suggestions.
pub struct SuggestionsIter<'a> {
    iter: std::slice::Iter<'a, (f64, String)>,
}

impl<'a> Iterator for SuggestionsIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, candidate)| candidate.as_str())
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

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
}
