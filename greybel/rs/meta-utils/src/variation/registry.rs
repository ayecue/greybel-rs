use indexmap::IndexMap;

use super::Variation;
use crate::error::{MetaError, Result};

#[derive(Debug, Clone, Default)]
pub struct VariationRegistry {
    groups: IndexMap<String, Vec<Variation>>,
}

impl VariationRegistry {
    pub fn add(
        &mut self,
        name: impl Into<String>,
        variations: impl IntoIterator<Item = Variation>,
    ) {
        let mut group = Vec::new();

        for variation in variations {
            push_unique(&mut group, variation);
        }

        self.groups.insert(name.into(), group);
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[Variation]> {
        self.groups.get(name).map(Vec::as_slice)
    }

    pub fn groups(&self) -> impl Iterator<Item = (&str, &[Variation])> {
        self.groups
            .iter()
            .map(|(name, group)| (name.as_str(), group.as_slice()))
    }

    /// Returns variations with each group expanded to unique members in `Variation` order.
    pub fn resolve(&self, variations: &[Variation]) -> Result<Vec<Variation>> {
        let mut resolved_variations = Vec::new();

        for variation in variations {
            let Some(name) = variation.group() else {
                push_unique(&mut resolved_variations, variation.clone());
                continue;
            };
            let group = self
                .get(name)
                .ok_or_else(|| MetaError::UnknownVariationGroup {
                    group: name.to_owned(),
                })?;

            for variation in group {
                push_unique(&mut resolved_variations, variation.clone());
            }
        }

        resolved_variations.sort();

        Ok(resolved_variations)
    }
}

fn push_unique(variations: &mut Vec<Variation>, variation: Variation) {
    if !variations.contains(&variation) {
        variations.push(variation);
    }
}
