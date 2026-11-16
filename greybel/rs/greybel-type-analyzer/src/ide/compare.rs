use std::mem;

const MAX_MATCHES: usize = 3;

/// Compares names with a name that isn't found, to find the ones it was probably meant to be.
pub(crate) struct NameComparison {
    name: String,
    lowercase: String,
    max_distance: usize,
}

impl NameComparison {
    /// A candidate matches when it differs only in case, or by at most a third of the name's length in edits (at
    /// least 1).
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            lowercase: name.to_lowercase(),
            max_distance: name.chars().count().max(3) / 3,
        }
    }

    /// The matching candidates, closest first.
    pub(crate) fn closest<'a>(
        &self,
        candidates: impl IntoIterator<Item = &'a str>,
    ) -> Vec<&'a str> {
        let mut matches: Vec<(usize, &str)> = candidates
            .into_iter()
            .filter_map(|candidate| Some((self.distance(candidate)?, candidate)))
            .collect();

        matches.sort_unstable();
        matches.dedup_by_key(|&mut (_, candidate)| candidate);
        matches.truncate(MAX_MATCHES);

        matches
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect()
    }

    fn distance(&self, candidate: &str) -> Option<usize> {
        if candidate == self.name {
            return None;
        }

        if candidate.to_lowercase() == self.lowercase {
            return Some(0);
        }

        let distance = Self::edit_distance(&self.name, candidate);

        (distance <= self.max_distance).then_some(distance)
    }

    /// Edit distance with adjacent transpositions (optimal string alignment), over characters.
    fn edit_distance(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let mut before: Vec<usize> = Vec::new();
        let mut previous: Vec<usize> = (0..=b.len()).collect();
        let mut current = vec![0; b.len() + 1];

        for i in 1..=a.len() {
            current[0] = i;

            for j in 1..=b.len() {
                let cost = usize::from(a[i - 1] != b[j - 1]);

                current[j] = (previous[j] + 1)
                    .min(current[j - 1] + 1)
                    .min(previous[j - 1] + cost);

                if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                    current[j] = current[j].min(before[j - 2] + 1);
                }
            }

            before = mem::replace(&mut previous, current.clone());
        }

        previous[b.len()]
    }
}

#[cfg(test)]
#[path = "tests/compare.rs"]
mod tests;
