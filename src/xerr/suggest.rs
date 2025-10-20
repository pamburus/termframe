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
    pub fn iter(&self) -> SuggestionsIter<'_> {
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
mod tests;
