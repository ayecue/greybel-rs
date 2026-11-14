use greyhack_world_storage::Schema;

use crate::storage::schema::{CORE_TABLES, WORLD_META_TABLES};

/// The schema of the core tables.
pub fn schema() -> Schema {
    CORE_TABLES
        .into_iter()
        .fold(Schema::new(), |schema, set| schema.with(set))
        .with(WORLD_META_TABLES)
}
