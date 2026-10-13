//! Serialized as `{"sources": [...], "mappings": [[first, last, source, line], ...]}`. The world persists this
//! in its `source_map` column, and the message hook's `SourceMap.cs` reads it.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Mapping, SourceMap};

#[derive(Serialize)]
struct BorrowedStorageForm<'m> {
    sources: &'m [String],
    mappings: Vec<[u32; 4]>,
}

#[derive(Deserialize)]
struct StorageForm {
    sources: Vec<String>,
    mappings: Vec<[u32; 4]>,
}

impl Serialize for SourceMap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        BorrowedStorageForm {
            sources: &self.sources,
            mappings: self
                .mappings
                .iter()
                .map(|mapping| [mapping.first, mapping.last, mapping.source, mapping.line])
                .collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SourceMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let form = StorageForm::deserialize(deserializer)?;
        let mappings = form
            .mappings
            .into_iter()
            .map(|[first, last, source, line]| Mapping::new(first, last, source, line))
            .collect();

        Ok(Self::new(form.sources, mappings))
    }
}
