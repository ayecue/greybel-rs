use meta_utils::Variation;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VariationGroup {
    pub(super) name: String,
    pub(super) variations: Vec<Variation>,
}

pub(super) fn groups() -> Vec<VariationGroup> {
    serde_json::from_str(&super::text("variations.json"))
        .unwrap_or_else(|error| panic!("data/variations.json: {error}"))
}
