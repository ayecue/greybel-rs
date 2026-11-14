use greyhack_world_storage::TableSet;

pub const PARENTS: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS parents (id INTEGER PRIMARY KEY);\n",
    &["parents"],
);
pub const CHILDREN: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS children (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES parents(id));\n\
     CREATE INDEX IF NOT EXISTS children_parent ON children (parent_id);\n",
    &["children"],
);
pub const SCRATCH: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS scratch (id INTEGER PRIMARY KEY);\n",
    &["scratch"],
)
.with_volatile("DELETE FROM scratch;\n");
