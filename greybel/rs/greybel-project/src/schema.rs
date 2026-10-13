use serde_json::Value;

use crate::manifest::Manifest;

impl Manifest {
    /// JSON Schema for `greybel.json`, including every setting default.
    #[must_use]
    pub fn schema() -> Value {
        serde_json::to_value(schemars::schema_for!(Manifest)).expect("a schema is JSON")
    }
}
