use std::cmp::Ordering;

const MIN_RELEVANCE: f64 = 0.75;

#[derive(Debug, Clone)]
pub struct Suggestions {
    wanted: String,
    candidates: Vec<(f64, String)>,
}

impl Suggestions {
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

        Self {
            wanted: wanted.to_owned(),
            candidates,
        }
    }

    pub fn iter(&self) -> SuggestionsIter {
        SuggestionsIter {
            iter: self.candidates.iter(),
        }
    }

    pub fn merge(self, other: Self) -> Result<Self, (Self, Self)> {
        if self.wanted != other.wanted {
            return Err((self, other));
        }

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

        Ok(Self {
            wanted: self.wanted,
            candidates,
        })
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

pub struct SuggestionsIter<'a> {
    iter: std::slice::Iter<'a, (f64, String)>,
}

impl<'a> Iterator for SuggestionsIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, candidate)| candidate.as_str())
    }
}
