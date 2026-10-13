mod conversion;

use std::cell::RefCell;
use std::collections::BTreeMap;

use duplex_protocol::{
    Direction, MessageMigration, MessagePlan, MessageSegment, MigrationPlan, Symbol,
};
use oxc_ast::ast::{BinaryOperator, ClassElement, Statement};
use oxc_span::SPAN;

use crate::ast::{ModifierContainer, Parameter, Ts};
use crate::codec::{CHANNEL_ERROR, UNAVAILABLE_METHOD};
use crate::types::TsTypeMapper;

pub(crate) const OLDER: &str = "older";
pub(crate) const NEWER: &str = "newer";
const MIGRATION: &str = "Migration";
const IMESSAGE: &str = "IMessage";
const MESSAGE_SEGMENT: &str = "MessageSegment";
const SEGMENT: &str = "segment";
pub(super) const RANGE_ERROR: &str = "RangeError";
pub(super) const OBJECT: &str = "Object";
const MESSAGE: &str = "message";
const UNUSED_MESSAGE: &str = "_message";
pub(super) const VALUE: &str = "value";
const TYPE: &str = "type";
pub(super) const ITEM: &str = "item";
pub(super) const ENTRY: &str = "entry";

pub(crate) struct MigrationClass<'p, 't, 'a> {
    ts: &'t Ts<'a>,
    plan: &'p MigrationPlan<'p, 'p>,
    name: String,
    conversions: RefCell<BTreeMap<(Direction, String), ClassElement<'a>>>,
}

impl<'p, 't, 'a> MigrationClass<'p, 't, 'a> {
    pub(crate) fn new(ts: &'t Ts<'a>, plan: &'p MigrationPlan<'p, 'p>) -> Self {
        let name = Symbol::Migration(
            plan.newer_protocol.protocol,
            plan.older_protocol.version.major,
        )
        .pascal();

        Self {
            ts,
            plan,
            name,
            conversions: RefCell::default(),
        }
    }

    pub(crate) fn declaration(&self) -> Statement<'a> {
        let ts = self.ts;
        let protocol = &self.plan.newer_protocol.protocol.name;
        let older = self.plan.older_protocol.version.major;
        let mut elements = vec![ts.constructor(
            Vec::new(),
            vec![ts.expression(ts.call(
                ts.super_(),
                vec![
                    ts.path(&format!("{OLDER}.{protocol}")),
                    ts.path(&format!("{NEWER}.{protocol}")),
                ],
            ))],
        )];

        for migration in &self.plan.messages {
            elements.extend(self.methods(migration));
        }

        elements.extend([self.type_of(true), self.type_of(false), self.rewrite()]);
        elements.extend(std::mem::take(&mut *self.conversions.borrow_mut()).into_values());

        let docs = ts.doc(format!(
            "Rewrites what clients of `{protocol}` {older}.x send and are sent, between {older}.x and {}.x: a method for each segment of a message both majors have, by default when their layouts and names are the same.",
            older + 1
        ));

        ts.class(
            docs,
            &self.name,
            true,
            Some((
                ts.id(MIGRATION),
                vec![
                    ts.type_of(&format!("{OLDER}.{protocol}")),
                    ts.type_of(&format!("{NEWER}.{protocol}")),
                ],
            )),
            elements,
        )
    }

    fn types(&self, newer: bool) -> TsTypeMapper<'t, 'a> {
        TsTypeMapper::in_namespace(self.ts, if newer { NEWER } else { OLDER })
    }

    fn message_path(newer: bool, plan: &MessagePlan<'_>) -> String {
        format!(
            "{}.{}",
            if newer { NEWER } else { OLDER },
            plan.message.name
        )
    }

    fn methods(&self, migration: &MessageMigration<'_, '_>) -> Vec<ClassElement<'a>> {
        let ts = self.ts;
        let message = migration.message;
        let from_newer = message.direction.from_newer();
        let name = Symbol::Migrate(migration.newer.message, MessageSegment::Message)
            .name()
            .camel();
        let from_type = ts.ty(&Self::message_path(from_newer, message.from), Vec::new());
        let to_path = Self::message_path(!from_newer, message.to);
        let mut methods = vec![if message.by_default {
            let properties = self.convert_properties(
                message.direction,
                &message.from.body,
                &message.to.body,
                MESSAGE,
            );
            let parameter = if message.from.body.properties().is_empty() {
                UNUSED_MESSAGE
            } else {
                MESSAGE
            };
            ts.method(
                SPAN,
                &name,
                vec![Parameter::new(parameter, from_type)],
                ts.ty(&to_path, Vec::new()),
                vec![ts.return_(Some(Self::construct(ts, &to_path, properties)))],
                ModifierContainer::NONE,
            )
        } else {
            ts.abstract_method(
                &name,
                vec![Parameter::new(MESSAGE, from_type)],
                ts.ty(&to_path, Vec::new()),
            )
        }];

        for (segment, rewrite) in &migration.segments {
            let rewrite = *rewrite;
            let name = Symbol::Migrate(migration.newer.message, *segment)
                .name()
                .camel();
            let from_type = self.types(rewrite.direction.from_newer()).of(rewrite.from);
            let to_type = self.types(!rewrite.direction.from_newer()).of(rewrite.to);

            methods.push(if rewrite.by_default {
                ts.method(
                    SPAN,
                    &name,
                    vec![Parameter::new(VALUE, from_type)],
                    to_type,
                    vec![ts.return_(Some(self.convert(
                        rewrite.direction,
                        rewrite.from,
                        rewrite.to,
                        &|| ts.id(VALUE),
                    )))],
                    ModifierContainer::NONE,
                )
            } else {
                ts.abstract_method(&name, vec![Parameter::new(VALUE, from_type)], to_type)
            });
        }

        methods
    }

    fn type_of(&self, older: bool) -> ClassElement<'a> {
        let ts = self.ts;
        let mut body: Vec<Statement<'a>> = self
            .plan
            .messages
            .iter()
            .map(|migration| {
                let (from, to) = if older {
                    (
                        Self::message_path(true, migration.newer),
                        Self::message_path(false, migration.older),
                    )
                } else {
                    (
                        Self::message_path(false, migration.older),
                        Self::message_path(true, migration.newer),
                    )
                };

                ts.if_(
                    ts.binary(ts.id(TYPE), BinaryOperator::StrictEquality, ts.path(&from)),
                    vec![ts.return_(Some(ts.path(&to)))],
                    None,
                )
            })
            .collect();

        body.push(ts.return_(Some(ts.undefined_literal())));

        ts.method(
            SPAN,
            if older { "olderType" } else { "newerType" },
            vec![Parameter::new(TYPE, ts.ty(IMESSAGE, Vec::new()))],
            ts.union(vec![ts.ty(IMESSAGE, Vec::new()), ts.undefined_type()]),
            body,
            ModifierContainer::OVERRIDE,
        )
    }

    fn rewrite(&self) -> ClassElement<'a> {
        let ts = self.ts;
        let mut body: Vec<Statement<'a>> = Vec::new();

        for migration in &self.plan.messages {
            let message = migration.message;
            let segments = std::iter::once((MessageSegment::Message, message.direction, None))
                .chain(
                    migration.segments.iter().map(|(segment, rewrite)| {
                        (*segment, rewrite.direction, Some(rewrite.from))
                    }),
                );

            for (segment, direction, from) in segments {
                let from_newer = direction.from_newer();
                let class = if from_newer {
                    migration.newer
                } else {
                    migration.older
                };
                let from_type = match from {
                    None => ts.ty(&Self::message_path(from_newer, message.from), Vec::new()),
                    Some(from) => self.types(from_newer).of(from),
                };
                let matches = ts.and(
                    ts.binary(
                        ts.id(TYPE),
                        BinaryOperator::StrictEquality,
                        ts.path(&Self::message_path(from_newer, class)),
                    ),
                    ts.binary(
                        ts.id(SEGMENT),
                        BinaryOperator::StrictEquality,
                        ts.path(&format!("{MESSAGE_SEGMENT}.{}", segment.pascal())),
                    ),
                );

                body.push(ts.if_(
                    matches,
                    vec![ts.return_(Some(ts.call_method(
                        ts.this(),
                        &Symbol::Migrate(migration.newer.message, segment).name().camel(),
                        vec![ts.as_(ts.id(VALUE), from_type)],
                    )))],
                    None,
                ));
            }
        }

        for migration in &self.plan.messages {
            for &segment in &migration.passthrough {
                let up = (migration.message.direction == Direction::Up) != segment.is_response();
                let class = if up { migration.older } else { migration.newer };
                let matches = ts.and(
                    ts.binary(
                        ts.id(TYPE),
                        BinaryOperator::StrictEquality,
                        ts.path(&Self::message_path(!up, class)),
                    ),
                    ts.binary(
                        ts.id(SEGMENT),
                        BinaryOperator::StrictEquality,
                        ts.path(&format!("{MESSAGE_SEGMENT}.{}", segment.pascal())),
                    ),
                );

                body.push(ts.if_(matches, vec![ts.return_(Some(ts.id(VALUE)))], None));
            }
        }

        body.push(ts.throw(ts.call_method(ts.id(CHANNEL_ERROR), UNAVAILABLE_METHOD, Vec::new())));

        ts.method(
            SPAN,
            "rewrite",
            vec![
                Parameter::new(SEGMENT, ts.ty(MESSAGE_SEGMENT, Vec::new())),
                Parameter::new(TYPE, ts.ty(IMESSAGE, Vec::new())),
                Parameter::new(VALUE, ts.unknown()),
            ],
            ts.unknown(),
            body,
            ModifierContainer::OVERRIDE,
        )
    }

    pub(crate) fn imports() -> [(&'static str, bool); 4] {
        [
            (CHANNEL_ERROR, false),
            (MESSAGE_SEGMENT, false),
            (MIGRATION, false),
            (IMESSAGE, true),
        ]
    }
}
