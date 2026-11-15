//! Registration of the tables a database has.

/// The tables one domain contributes: their `CREATE TABLE` and `CREATE INDEX` statements, the
/// table names they create, and what a database deletes from them when it opens.
///
/// Statements must be idempotent (`IF NOT EXISTS`), because a persisted database runs them again
/// at every open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableSet {
    schema: &'static str,
    tables: &'static [&'static str],
    volatile: &'static str,
}

impl TableSet {
    /// Creates a set from the statements in `schema` and the names of the `tables` they create,
    /// in creation order.
    #[must_use]
    pub const fn new(schema: &'static str, tables: &'static [&'static str]) -> Self {
        Self {
            schema,
            tables,
            volatile: "",
        }
    }

    /// Creates a set with no tables that only clears rows of other sets when a database opens.
    #[must_use]
    pub const fn volatile(clear: &'static str) -> Self {
        Self::new("", &[]).with_volatile(clear)
    }

    /// Returns this set that also runs the statements in `clear` after the schema of every set
    /// exists each time a database opens, to delete rows that must not outlive the program.
    #[must_use]
    pub const fn with_volatile(mut self, clear: &'static str) -> Self {
        self.volatile = clear;

        self
    }

    /// Returns the `CREATE TABLE` and `CREATE INDEX` statements.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        self.schema
    }

    /// Returns the names of the tables the statements create.
    #[must_use]
    pub const fn tables(&self) -> &'static [&'static str] {
        self.tables
    }

    /// Returns the statements run when a database opens.
    #[must_use]
    pub const fn volatile_sql(&self) -> &'static str {
        self.volatile
    }
}

use crate::{DomainSchema, SCHEMA_VERSIONS};

/// The table sets of a database in dependency order.
///
/// Domain tables follow their declared dependencies; ties use registration order. Ownerless sets
/// use registration order, so a set may reference tables registered before it.
#[derive(Debug, Clone, Default)]
pub struct Schema {
    sets: Vec<TableSet>,
    ownerless: Vec<TableSet>,
    domains: Vec<DomainSchema>,
}

impl Schema {
    /// Creates a schema with no tables.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sets: Vec::new(),
            ownerless: Vec::new(),
            domains: Vec::new(),
        }
    }

    /// Registers `set` after the sets registered before it.
    ///
    /// # Panics
    ///
    /// Panics if `set` names a table that another set already registered.
    pub fn register(&mut self, set: TableSet) -> &mut Self {
        for table in set.tables {
            assert!(
                !self.sets.iter().any(|other| other.tables.contains(table)),
                "table `{table}` is registered twice"
            );
        }

        self.sets.push(set);
        self.ownerless.push(set);

        self
    }

    /// Returns this schema with `set` registered after the sets registered before it.
    ///
    /// # Panics
    ///
    /// Panics if `set` names a table that another set already registered.
    #[must_use]
    pub fn with(mut self, set: TableSet) -> Self {
        self.register(set);

        self
    }

    /// Registers a domain after the domains and sets registered before it.
    ///
    /// # Panics
    ///
    /// Panics if the domain name or one of its tables is already registered.
    pub fn register_domain(&mut self, domain: DomainSchema) -> &mut Self {
        assert!(
            !self.domains.iter().any(|other| other.name == domain.name),
            "domain `{}` is registered twice",
            domain.name
        );

        for set in &domain.tables {
            for table in set.tables {
                assert!(
                    !self.sets.iter().any(|other| other.tables.contains(table)),
                    "table `{table}` is registered twice"
                );
            }

            self.sets.push(*set);
        }

        self.domains.push(domain);
        self.order_domains();

        self
    }

    fn order_domains(&mut self) {
        if self.domains.iter().any(|domain| {
            domain
                .depends_on
                .iter()
                .any(|dependency| !self.domains.iter().any(|other| other.name == *dependency))
        }) {
            return;
        }

        let mut queue = std::mem::take(&mut self.domains);
        let mut result = Vec::with_capacity(queue.len());

        while !queue.is_empty() {
            let next = queue.iter().position(|domain| {
                domain.depends_on.iter().all(|dependency| {
                    result
                        .iter()
                        .any(|other: &DomainSchema| other.name == *dependency)
                })
            });
            let Some(next) = next else {
                let names = queue
                    .iter()
                    .map(|domain| domain.name)
                    .collect::<Vec<_>>()
                    .join("`, `");

                panic!("domain schema dependency cycle involving `{names}`");
            };

            result.push(queue.remove(next));
        }

        self.domains = result;
        self.sets = self
            .ownerless
            .iter()
            .chain(self.domains.iter().flat_map(|domain| domain.tables.iter()))
            .copied()
            .collect();
    }

    fn validate_dependencies(&self) {
        for domain in &self.domains {
            for dependency in domain.depends_on {
                assert!(
                    self.domains.iter().any(|other| other.name == *dependency),
                    "domain `{}` depends on unknown domain `{dependency}`",
                    domain.name
                );
            }
        }
    }

    /// Returns this schema with `domain` registered after its dependencies.
    #[must_use]
    pub fn with_domain(mut self, domain: DomainSchema) -> Self {
        self.register_domain(domain);

        self
    }

    /// Returns registered domain schemas in layer order.
    #[must_use]
    pub fn domains(&self) -> &[DomainSchema] {
        self.validate_dependencies();

        &self.domains
    }

    /// Returns sets not owned by a versioned domain, in registration order.
    #[must_use]
    pub fn ownerless_sets(&self) -> &[TableSet] {
        &self.ownerless
    }

    /// Returns the registered sets in registration order.
    #[must_use]
    pub fn sets(&self) -> &[TableSet] {
        self.validate_dependencies();

        &self.sets
    }

    /// Returns the `CREATE TABLE` and `CREATE INDEX` statements of every set in registration
    /// order, executed when a database opens.
    #[must_use]
    pub fn sql(&self) -> String {
        self.validate_dependencies();

        std::iter::once(&SCHEMA_VERSIONS)
            .chain(&self.sets)
            .map(|set| set.schema)
            .collect()
    }

    /// Returns the table names of every set in registration order: parents before dependent rows.
    #[must_use]
    pub fn tables(&self) -> Vec<&'static str> {
        self.validate_dependencies();

        std::iter::once(&SCHEMA_VERSIONS)
            .chain(&self.sets)
            .flat_map(|set| set.tables)
            .copied()
            .collect()
    }

    /// Returns the statements of every set that clear volatile rows, in registration order,
    /// executed after [`Schema::sql`] each time a database opens.
    #[must_use]
    pub fn volatile_sql(&self) -> String {
        self.validate_dependencies();

        self.sets.iter().map(|set| set.volatile).collect()
    }
}
