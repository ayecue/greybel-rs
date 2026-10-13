mod encoding;
mod fields;
mod migration;
mod protocol;
mod requirements;
mod types;

pub use encoding::Encoding;
pub use fields::{Bit, BlockPlan, FieldPlan, FieldValue, FieldsPlan, Presence};
pub use migration::{Direction, MessageMigration, MigrationError, MigrationPlan, Rewrite};
pub use protocol::{
    MessagePlan, ProtocolPlan, ResponseBodyPlan, ResponsePlan, RolePlan, StreamPlan, VersionPlan,
};
pub use types::{ConstantPlan, MemberPlan, PayloadPlan, TypeContent, TypePlan, VariantPlan};

use crate::model::Definition;
use crate::versions::{AdditionTable, MinorVersionTable};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Source<'d> {
    pub(crate) definition: &'d Definition,
    pub(crate) additions: &'d AdditionTable,
}

/// Generation plan for a major's latest minor.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan<'d> {
    /// Source definition.
    pub definition: &'d Definition,
    pub types: Vec<TypePlan<'d>>,
    pub constants: Vec<ConstantPlan<'d>>,
    /// Protocol plan when the definition declares a protocol.
    pub protocol: Option<ProtocolPlan<'d>>,
    /// Declared types with direct or transitive validation requirements.
    pub requirement_types: std::collections::BTreeSet<&'d str>,
}

impl<'d> Plan<'d> {
    /// Resolves the generation plan for the latest minor in `major`.
    #[must_use]
    pub fn of(major: MinorVersionTable<'d>) -> Self {
        let latest = major.latest();
        let definition = &latest.definition;
        let source = Source {
            definition,
            additions: &latest.additions,
        };

        Self {
            definition,
            types: definition
                .types()
                .iter()
                .map(|declaration| TypePlan::of(source, declaration))
                .collect(),
            constants: definition
                .constants()
                .iter()
                .map(|constant| ConstantPlan::of(definition, constant))
                .collect(),
            protocol: definition
                .protocol()
                .map(|protocol| ProtocolPlan::of(major, protocol)),
            requirement_types: Self::requirement_types(definition),
        }
    }
}
