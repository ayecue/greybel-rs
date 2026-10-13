mod imports;
mod protocol;

use std::fmt::{self, Display, Formatter};

use duplex_protocol::{MigrationPlan, MinorVersionTable, Plan, Role, Status};
use oxc_allocator::Allocator;

use crate::ast::Ts;
use crate::codec::Codec;
use crate::declaration::{MigrationClass, RoleDeclarationBuilder, TypeDeclarationBuilder};
use crate::types::TsTypeMapper;

const CORE: &str = "duplex-core";
const GENERATOR: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Content {
    Types,
    Protocol,
    Role(Role),
    Migration,
}

#[derive(Clone, Copy)]
struct Migration<'a> {
    older: MinorVersionTable<'a>,
    older_module: &'a str,
    newer_module: &'a str,
}

/// TypeScript module for one definition major version.
pub struct Module<'a> {
    major: MinorVersionTable<'a>,
    content: Content,
    core: &'a str,
    core_types: Option<&'a str>,
    core_message: Option<&'a str>,
    types: Option<&'a str>,
    protocol: Option<&'a str>,
    migration: Option<Migration<'a>>,
}

impl<'a> Module<'a> {
    /// Returns a module declaring major-version types and binary codecs.
    #[must_use]
    pub fn types(major: MinorVersionTable<'a>) -> Self {
        Self::new(major, Content::Types)
    }

    /// Returns a module declaring major-version types, protocol, messages, and peers.
    ///
    /// # Panics
    ///
    /// Panics if the major lacks a supported protocol.
    #[must_use]
    pub fn protocol(major: MinorVersionTable<'a>) -> Self {
        Self::with_protocol_types(major, Content::Protocol)
    }

    /// Returns a module declaring the channel and router for `role`.
    ///
    /// # Panics
    ///
    /// Panics if the major lacks a supported protocol.
    #[must_use]
    pub fn role(major: MinorVersionTable<'a>, role: Role) -> Self {
        Self::with_protocol_types(major, Content::Role(role))
    }

    fn with_protocol_types(major: MinorVersionTable<'a>, content: Content) -> Self {
        let latest = major.latest();

        assert!(
            latest.definition.protocol().is_some(),
            "{} declares a protocol",
            latest.version
        );
        assert!(
            *latest.definition.status() != Status::Unavailable,
            "{} is supported",
            latest.version
        );

        Self::new(major, content)
    }

    /// Returns a module converting messages between consecutive protocol majors.
    ///
    /// # Panics
    ///
    /// Panics if the majors are nonconsecutive or cannot migrate.
    #[must_use]
    pub fn migration(
        older: MinorVersionTable<'a>,
        newer: MinorVersionTable<'a>,
        older_module: &'a str,
        newer_module: &'a str,
    ) -> Self {
        assert_eq!(
            older.major() + 1,
            newer.major(),
            "the majors are consecutive"
        );
        if let Err(error) = MigrationPlan::of(&Plan::of(older), &Plan::of(newer)) {
            panic!("{error}");
        }

        Self {
            migration: Some(Migration {
                older,
                older_module,
                newer_module,
            }),
            ..Self::with_protocol_types(newer, Content::Migration)
        }
    }

    fn new(major: MinorVersionTable<'a>, content: Content) -> Self {
        Self {
            major,
            content,
            core: CORE,
            core_types: None,
            core_message: None,
            types: None,
            protocol: None,
            migration: None,
        }
    }

    /// Sets the `duplex-core` import path.
    #[must_use]
    pub fn with_core(mut self, core: &'a str) -> Self {
        self.core = core;

        self
    }

    /// Sets a separate core-type import path for generated core modules whose index re-exports those types.
    #[must_use]
    pub fn with_core_types(mut self, core_types: &'a str) -> Self {
        self.core_types = Some(core_types);

        self
    }

    /// Sets a separate import path for `Message` and generated-class `BODY` keys. Generated core modules use this
    /// instead of importing them from the main core module.
    #[must_use]
    pub fn with_core_message(mut self, core_message: &'a str) -> Self {
        self.core_message = Some(core_message);

        self
    }

    /// Imports declared types from `types` rather than generating them.
    #[must_use]
    pub fn with_types(mut self, types: &'a str) -> Self {
        self.types = Some(types);

        self
    }

    /// Imports and re-exports the protocol from `protocol` rather than declaring it.
    #[must_use]
    pub fn with_protocol(mut self, protocol: &'a str) -> Self {
        self.protocol = Some(protocol);

        self
    }

    fn declares_protocol(&self) -> bool {
        match self.content {
            Content::Types | Content::Migration => false,
            Content::Protocol => true,
            Content::Role(_) => self.protocol.is_none(),
        }
    }

    fn declares_types(&self) -> bool {
        match self.content {
            Content::Types => true,
            Content::Migration => false,
            Content::Protocol | Content::Role(_) => {
                self.declares_protocol() && self.types.is_none()
            }
        }
    }

    fn plan(&self) -> Plan<'a> {
        Plan::of(self.major)
    }

    fn header(&self) -> String {
        let latest = self.major.latest();
        let source = match (self.content, latest.definition.protocol()) {
            (Content::Protocol, Some(protocol)) => {
                format!("the protocol `{}` {}", protocol.name, latest.version)
            }
            (Content::Role(role), Some(protocol)) => {
                format!(
                    "the protocol `{}` {} for its {}",
                    protocol.name,
                    latest.version,
                    role.keyword()
                )
            }
            (Content::Migration, Some(protocol)) => format!(
                "the protocol `{}`, migrating {}.x up to {}.x",
                protocol.name,
                latest.version.major - 1,
                latest.version.major
            ),
            _ => format!("the types of a definition {}", latest.version),
        };

        ["Generated by", GENERATOR, "from", &source].join(" ") + "; do not edit."
    }
}

impl Display for Module<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let plan = self.plan();
        let allocator = Allocator::default();
        let ts = Ts::new(&allocator);
        let codec = Codec::new(&ts, &plan);

        ts.header(&self.header());

        let mut body = self.imports(&ts, &plan);

        if self.declares_types() || self.declares_protocol() {
            body.extend(codec.patterns());
        }

        if self.declares_types() {
            let types = TsTypeMapper::new(&ts);

            for constant in &plan.constants {
                let value = types.value(&constant.value, &constant.ty);

                body.push(ts.exported_const(
                    ts.docs(&constant.constant.docs),
                    &constant.constant.name,
                    value,
                ));
            }

            let declarations = TypeDeclarationBuilder::new(&ts, &codec);

            for declaration in &plan.types {
                body.extend(declarations.declaration(declaration));
            }
        }

        if let Some(migration) = self.migration {
            let older = Plan::of(migration.older);
            let migration = MigrationPlan::of(&older, &plan)
                .expect("`Module::migration` checked the majors can be migrated between");

            body.push(MigrationClass::new(&ts, &migration).declaration());
        }

        if let Some(protocol) = plan.protocol.as_ref().filter(|_| self.migration.is_none()) {
            if self.declares_protocol() {
                body.extend(Self::protocol_declarations(&ts, &codec, protocol));
            }

            if let Content::Role(role) = self.content {
                body.extend(RoleDeclarationBuilder::new(&ts, protocol.role(role)).declarations());
            }
        }

        f.write_str(&ts.print(body))
    }
}
