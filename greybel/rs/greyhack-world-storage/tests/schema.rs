use greyhack_world_storage::{SCHEMA_VERSIONS, Schema, TableSet};

use crate::support::schema::{CHILDREN, PARENTS, SCRATCH};

#[test]
fn should_assemble_the_schema_in_registration_order() {
    // Arrange
    let mut schema = Schema::new();

    // Act
    schema
        .register(PARENTS)
        .register(CHILDREN)
        .register(SCRATCH);

    // Assert
    assert_eq!(
        schema.sql(),
        format!(
            "{}{}{}{}",
            SCHEMA_VERSIONS.schema(),
            PARENTS.schema(),
            CHILDREN.schema(),
            SCRATCH.schema()
        )
    );
    assert_eq!(
        schema.tables(),
        ["schema_versions", "parents", "children", "scratch"]
    );
    assert_eq!(schema.sets().len(), 3);
}

#[test]
fn should_follow_a_different_registration_order() {
    // Arrange
    let schema = Schema::new().with(SCRATCH).with(PARENTS);

    // Act
    let tables = schema.tables();

    // Assert
    assert_eq!(tables, ["schema_versions", "scratch", "parents"]);
}

#[test]
fn should_collect_volatile_statements_in_registration_order() {
    // Arrange
    let other = TableSet::volatile("DELETE FROM parents;\n");
    let schema = Schema::new().with(SCRATCH).with(PARENTS).with(other);

    // Act
    let volatile = schema.volatile_sql();

    // Assert
    assert_eq!(volatile, "DELETE FROM scratch;\nDELETE FROM parents;\n");
    assert_eq!(other.tables(), &[] as &[&str]);
    assert_eq!(other.schema(), "");
}

#[test]
fn should_have_no_volatile_statements_without_volatile_sets() {
    // Arrange
    let schema = Schema::new().with(PARENTS).with(CHILDREN);

    // Act
    let volatile = schema.volatile_sql();

    // Assert
    assert_eq!(volatile, "");
}

#[test]
#[should_panic(expected = "table `parents` is registered twice")]
fn should_reject_a_table_registered_twice() {
    // Arrange
    let mut schema = Schema::new();

    schema.register(PARENTS);

    // Act
    schema.register(PARENTS);
}
