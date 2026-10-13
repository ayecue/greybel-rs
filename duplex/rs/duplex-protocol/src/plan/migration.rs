use std::collections::HashSet;

use super::{
    Encoding, FieldsPlan, MessagePlan, PayloadPlan, Plan, ProtocolPlan, TypeContent, TypePlan,
};
use crate::model::Role;
use crate::names::MessageSegment;

/// Message value conversion direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    /// Converts client messages from older to newer major.
    Up,
    /// Converts provider messages from newer to older major.
    Down,
}

impl Direction {
    #[must_use]
    pub fn back(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }

    #[must_use]
    pub fn from_newer(self) -> bool {
        self == Self::Down
    }
}

/// Value conversion between major versions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rewrite<T> {
    /// Conversion direction.
    pub direction: Direction,
    /// Source value.
    pub from: T,
    /// Destination value.
    pub to: T,
    /// Whether matching layout and names allow automatic conversion.
    pub by_default: bool,
}

/// Major-version migration error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MigrationError {
    /// At least one major has no protocol declaration.
    #[error("a migration is between majors that both declare a protocol")]
    NoProtocol,
    /// Handshake payload differs between majors.
    #[error(
        "the handshake's `{0}` payload differs between the majors; a provider reads and writes it as its own major's, so it can't change in a major its migrations reach"
    )]
    HandshakeMismatch(&'static str),
}

/// Migration plan for consecutive protocol majors.
#[derive(Debug, Clone, PartialEq)]
pub struct MigrationPlan<'p, 'd> {
    /// Older major plan.
    pub older: &'p Plan<'d>,
    /// Newer major plan.
    pub newer: &'p Plan<'d>,
    /// Older major protocol plan.
    pub older_protocol: &'p ProtocolPlan<'d>,
    /// Newer major protocol plan.
    pub newer_protocol: &'p ProtocolPlan<'d>,
    /// Migrations for messages shared by both majors.
    pub messages: Vec<MessageMigration<'p, 'd>>,
}

/// Conversions for a shared message and its segments.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageMigration<'p, 'd> {
    /// Older major message plan.
    pub older: &'p MessagePlan<'d>,
    /// Newer major message plan.
    pub newer: &'p MessagePlan<'d>,
    /// Initial message field conversion.
    pub message: Rewrite<&'p MessagePlan<'d>>,
    /// Conversions for other value segments shared by both majors.
    pub segments: Vec<(MessageSegment, Rewrite<&'p Encoding<'d>>)>,
    /// Shared structural segments forwarded unchanged.
    pub passthrough: Vec<MessageSegment>,
}

impl<'p, 'd> MessageMigration<'p, 'd> {
    #[must_use]
    pub fn segment(&self, segment: MessageSegment) -> Option<&Rewrite<&'p Encoding<'d>>> {
        self.segments
            .iter()
            .find(|(of, _)| *of == segment)
            .map(|(_, rewrite)| rewrite)
    }
}

impl<'p, 'd> MigrationPlan<'p, 'd> {
    /// Builds a migration plan for two protocol majors.
    ///
    /// # Errors
    ///
    /// Returns an error if either major lacks a protocol or a handshake payload changed.
    pub fn of(older: &'p Plan<'d>, newer: &'p Plan<'d>) -> Result<Self, MigrationError> {
        let (Some(older_protocol), Some(newer_protocol)) =
            (older.protocol.as_ref(), newer.protocol.as_ref())
        else {
            return Err(MigrationError::NoProtocol);
        };
        let mut plan = Self {
            older,
            newer,
            older_protocol,
            newer_protocol,
            messages: Vec::new(),
        };

        plan.messages = older_protocol
            .messages
            .iter()
            .filter_map(|older| {
                let newer = newer_protocol.messages.iter().find(|newer| {
                    let name = newer
                        .message
                        .renames
                        .as_ref()
                        .unwrap_or(&newer.message.name);

                    *name == older.message.name && newer.message.receiver == older.message.receiver
                })?;
                let direction = match older.message.receiver {
                    Role::Provider => Direction::Up,
                    Role::Client => Direction::Down,
                };
                let segments = MessageSegment::ALL
                    .into_iter()
                    .filter_map(|segment| {
                        let (older, newer) = (older.segment(segment)?, newer.segment(segment)?);
                        let direction = if segment.is_response() {
                            direction.back()
                        } else {
                            direction
                        };
                        let same = plan.same(older, newer);

                        Some((segment, Self::rewrite(direction, older, newer, same)))
                    })
                    .collect();
                let passthrough = MessageSegment::ALL
                    .into_iter()
                    .filter(|&segment| {
                        older.has_segment(segment)
                            && newer.has_segment(segment)
                            && older.segment(segment).is_none()
                            && newer.segment(segment).is_none()
                            && segment != MessageSegment::Message
                    })
                    .collect();
                let by_default =
                    plan.same_fields(&older.body, &newer.body, &mut HashSet::new(), false);

                Some(MessageMigration {
                    older,
                    newer,
                    message: Self::rewrite(direction, older, newer, by_default),
                    segments,
                    passthrough,
                })
            })
            .collect();
        plan.verify_handshake()?;

        Ok(plan)
    }

    fn verify_handshake(&self) -> Result<(), MigrationError> {
        let (older, newer) = (self.older_protocol, self.newer_protocol);
        let payloads = [
            ("open", &older.open, &newer.open),
            ("ack", &older.ack, &newer.ack),
            ("refuse", &older.refusal, &newer.refusal),
        ];

        for (payload, older, newer) in payloads {
            let alike = match (older, newer) {
                (None, None) => true,
                (Some(older), Some(newer)) => self.same_in(older, newer, &mut HashSet::new(), true),
                _ => false,
            };

            if !alike {
                return Err(MigrationError::HandshakeMismatch(payload));
            }
        }

        Ok(())
    }

    fn rewrite<T>(direction: Direction, older: T, newer: T, by_default: bool) -> Rewrite<T> {
        let (from, to) = if direction.from_newer() {
            (newer, older)
        } else {
            (older, newer)
        };

        Rewrite {
            direction,
            from,
            to,
            by_default,
        }
    }

    #[must_use]
    pub fn declaration(&self, newer: bool, name: &str) -> Option<&'p TypePlan<'d>> {
        let plan = if newer { self.newer } else { self.older };

        plan.types.iter().find(|plan| plan.name() == name)
    }

    #[must_use]
    pub fn same(&self, older: &Encoding<'_>, newer: &Encoding<'_>) -> bool {
        self.same_in(older, newer, &mut HashSet::new(), false)
    }

    fn same_in(
        &self,
        older: &Encoding<'_>,
        newer: &Encoding<'_>,
        path_names: &mut HashSet<String>,
        exact: bool,
    ) -> bool {
        match (older, newer) {
            (Encoding::Bool, Encoding::Bool)
            | (Encoding::Char, Encoding::Char)
            | (Encoding::String(_), Encoding::String(_))
            | (Encoding::Bytes(_), Encoding::Bytes(_)) => true,
            (Encoding::Number(older, _), Encoding::Number(newer, _)) => older == newer,
            (Encoding::List(older, _), Encoding::List(newer, _))
            | (Encoding::Record(older, _), Encoding::Record(newer, _))
            | (Encoding::Nullable(older), Encoding::Nullable(newer)) => {
                self.same_in(older, newer, path_names, exact)
            }
            (Encoding::Struct(older), Encoding::Struct(newer))
            | (Encoding::Union(older), Encoding::Union(newer))
            | (Encoding::Enumeration(older), Encoding::Enumeration(newer)) => {
                if older != newer {
                    return false;
                }

                if !path_names.insert((*older).to_owned()) {
                    return true;
                }

                match (
                    self.declaration(false, older),
                    self.declaration(true, newer),
                ) {
                    (Some(older), Some(newer)) => {
                        self.same_content(&older.content, &newer.content, path_names, exact)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn same_content(
        &self,
        older: &TypeContent<'_>,
        newer: &TypeContent<'_>,
        path_names: &mut HashSet<String>,
        exact: bool,
    ) -> bool {
        match (older, newer) {
            (TypeContent::Struct(older), TypeContent::Struct(newer)) => {
                self.same_fields(older, newer, path_names, exact)
            }
            (TypeContent::Union(older), TypeContent::Union(newer)) => {
                older.len() == newer.len()
                    && older.iter().zip(newer).all(|(older, newer)| {
                        older.variant.name == newer.variant.name
                            && match (&older.payload, &newer.payload) {
                                (PayloadPlan::Unit, PayloadPlan::Unit) => true,
                                (PayloadPlan::Value(older), PayloadPlan::Value(newer)) => {
                                    self.same_in(older, newer, path_names, exact)
                                }
                                (PayloadPlan::Fields(older), PayloadPlan::Fields(newer)) => {
                                    self.same_fields(older, newer, path_names, exact)
                                }
                                _ => false,
                            }
                    })
            }
            (TypeContent::Enumeration(older), TypeContent::Enumeration(newer)) => {
                older.len() == newer.len()
                    && older.iter().zip(newer).all(|(older, newer)| {
                        older.member.name == newer.member.name && older.value == newer.value
                    })
            }
            _ => false,
        }
    }

    fn same_fields(
        &self,
        older: &FieldsPlan<'_>,
        newer: &FieldsPlan<'_>,
        path_names: &mut HashSet<String>,
        exact: bool,
    ) -> bool {
        let older: Vec<_> = older.fields().collect();
        let newer: Vec<_> = newer.fields().collect();
        let common_len = older.len().min(newer.len());
        let mut remainder = older[common_len..].iter().chain(&newer[common_len..]);
        let remainder_alike = if exact {
            older.len() == newer.len()
        } else {
            remainder.all(|field| field.can_be_left_out())
        };

        remainder_alike
            && older.iter().zip(&newer).all(|(older, newer)| {
                older.field.name == newer.field.name
                    && older.field.optional == newer.field.optional
                    && older.field.flatten == newer.field.flatten
                    && self.same_in(&older.ty, &newer.ty, path_names, exact)
            })
    }
}
