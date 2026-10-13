use super::Container;
use crate::description::DescriptionTable;
use crate::error::Result;
use crate::signature::{Signature, SignaturePayload};
use crate::variation::Variation;

impl Container {
    pub fn add_variations(
        &mut self,
        name: impl Into<String>,
        variations: impl IntoIterator<Item = Variation>,
    ) {
        self.variations.add(name, variations);
    }

    pub fn exclude_from_search(&mut self, type_name: impl Into<String>) {
        self.index.take();
        self.search_exclusions.insert(type_name.into());
    }

    pub fn include_in_search(&mut self, type_name: &str) {
        self.index.take();
        self.search_exclusions.remove(type_name);
    }

    pub fn add_signature(&mut self, payload: SignaturePayload) {
        self.signature_mut(payload.type_name.clone()).merge(payload);
    }

    pub fn add_signature_json(&mut self, json: &str) -> Result<()> {
        self.add_signature(serde_json::from_str(json)?);

        Ok(())
    }

    pub fn add_descriptions(
        &mut self,
        language: &str,
        type_name: impl Into<String>,
        descriptions: DescriptionTable,
    ) {
        self.signature_mut(type_name.into())
            .merge_descriptions(language, descriptions);
    }

    pub fn add_descriptions_json(
        &mut self,
        language: &str,
        type_name: impl Into<String>,
        json: &str,
    ) -> Result<()> {
        self.add_descriptions(language, type_name, serde_json::from_str(json)?);

        Ok(())
    }

    fn signature_mut(&mut self, type_name: String) -> &mut Signature {
        self.index.take();

        self.signatures
            .entry(type_name)
            .or_insert_with_key(|name| Signature::new(name.clone()))
    }
}
