use duplex_protocol::{
    DocumentationRef, Encoding, FieldsPlan, MemberPlan, PayloadPlan, TagList, TypeContent,
    TypePlan, VariantPlan,
};
use oxc_ast::ast::{BinaryOperator, Statement};
use oxc_span::SPAN;

use super::members::MemberBuilder;
use super::tags::{MEMBER_TAGS, MEMBERS, TAG, TAGS, TagExpressionContainer};
use super::{BODY, BODY_KEY, FIELDS, VALUE};
use crate::ast::{EnumMember, InterfaceProperty, ModifierContainer, Parameter, Ts};
use crate::codec::Codec;
use crate::types::TsTypeMapper;

const INDEX: &str = "index";
const RANGE_ERROR: &str = "RangeError";

pub(crate) struct TypeDeclarationBuilder<'t, 'a> {
    ts: &'t Ts<'a>,
    codec: &'t Codec<'t, 'a>,
}

impl<'t, 'a> TypeDeclarationBuilder<'t, 'a> {
    pub(crate) fn new(ts: &'t Ts<'a>, codec: &'t Codec<'t, 'a>) -> Self {
        Self { ts, codec }
    }

    pub(crate) fn declaration(&self, plan: &TypePlan<'_>) -> Vec<Statement<'a>> {
        let docs = &plan.declaration.docs;
        let tags = &plan.declaration.tags;

        match &plan.content {
            TypeContent::Struct(fields) => self.structure(docs, tags, plan.name(), fields),
            TypeContent::Union(variants) => self.union(docs, tags, plan.name(), variants),
            TypeContent::Enumeration(members) => self.enumeration(docs, tags, plan.name(), members),
        }
    }

    fn tags(&self) -> TagExpressionContainer<'t, 'a> {
        TagExpressionContainer::new(self.ts, self.codec.plan().definition)
    }

    fn structure(
        &self,
        docs: &DocumentationRef,
        tags: &TagList,
        name: &str,
        fields: &FieldsPlan<'_>,
    ) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let members = MemberBuilder::new(self.codec, ts, fields, name.to_owned());
        let expressions = self.tags();
        let mut elements = expressions.statics(tags, expressions.fields(fields), false);

        elements.extend(members.body(false));
        elements.extend(members.getters());
        elements.push(members.write(false, Vec::new()));
        elements.push(members.read());

        let mut statements = vec![ts.class(ts.docs(docs), name, false, None, elements)];
        let interfaces = members.interfaces();

        if !interfaces.is_empty() {
            statements.push(ts.namespace(name, interfaces));
        }

        statements
    }

    fn union(
        &self,
        docs: &DocumentationRef,
        tags: &TagList,
        name: &str,
        variants: &[VariantPlan<'_>],
    ) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let cases = variants
            .iter()
            .map(|plan| {
                let variant = format!("{name}.{}", plan.variant.name);
                let read = ts.call_method(ts.path(&variant), "read", vec![ts.id("reader")]);

                (ts.number_literal(plan.index), vec![ts.return_(Some(read))])
            })
            .collect();
        let unknown = ts.throw(ts.new_(
            ts.id(RANGE_ERROR),
            vec![ts.template(
                &["no variant ", &format!(" of `{name}`")],
                vec![ts.id(INDEX)],
            )],
        ));
        let read = ts.method(
            SPAN,
            "read",
            vec![MemberBuilder::reader_parameter(ts)],
            ts.ty(name, Vec::new()),
            vec![
                ts.const_(
                    INDEX,
                    None,
                    ts.call_method(ts.id("reader"), "u32", Vec::new()),
                ),
                ts.switch(ts.id(INDEX), cases, vec![unknown]),
            ],
            ModifierContainer::STATIC,
        );
        let elements = vec![
            ts.property(
                SPAN,
                TAGS,
                Some(self.tags().list(TAG)),
                Some(self.tags().tags(tags)),
                ModifierContainer::STATIC_READONLY,
            ),
            ts.abstract_method(
                "write",
                vec![MemberBuilder::writer_parameter(ts)],
                ts.void(),
            ),
            read,
        ];
        let namespace = variants
            .iter()
            .flat_map(|plan| self.variant(name, plan))
            .collect();

        vec![
            ts.class(ts.docs(docs), name, true, None, elements),
            ts.namespace(name, namespace),
        ]
    }

    fn variant(&self, union: &str, plan: &VariantPlan<'_>) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let name = &plan.variant.name;
        let class = format!("{union}.{name}");
        let prefix: Vec<Statement<'a>> = self
            .codec
            .version_guard(plan.since)
            .into_iter()
            .chain([ts.expression(ts.call_method(
                ts.id("writer"),
                "u32",
                vec![ts.number_literal(plan.index)],
            ))])
            .collect();
        let span = ts.docs(&plan.variant.docs);
        let empty = FieldsPlan { blocks: Vec::new() };
        let fields = match &plan.payload {
            PayloadPlan::Value(ty) => {
                return self.value_variant(span, union, plan, &class, prefix, ty);
            }
            PayloadPlan::Unit => &empty,
            PayloadPlan::Fields(fields) => fields,
        };
        let members = MemberBuilder::new(self.codec, ts, fields, class);
        let expressions = self.tags();
        let mut elements =
            expressions.statics(&plan.variant.tags, expressions.fields(fields), false);

        elements.extend(members.body(true));
        elements.extend(members.getters());
        elements.push(members.write(true, prefix));
        elements.push(members.read());

        let mut statements = vec![ts.class(
            span,
            name,
            false,
            Some((ts.id(union), Vec::new())),
            elements,
        )];
        let interfaces = members.interfaces();

        if !interfaces.is_empty() {
            statements.push(ts.namespace(name, interfaces));
        }

        statements
    }

    fn value_variant(
        &self,
        span: oxc_span::Span,
        union: &str,
        plan: &VariantPlan<'_>,
        class: &str,
        prefix: Vec<Statement<'a>>,
        ty: &Encoding<'_>,
    ) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let name = &plan.variant.name;
        let types = TsTypeMapper::new(ts);
        let fields_type = format!("{class}.{FIELDS}");
        let expressions = self.tags();

        self.codec.reset_locals();

        let mut write = prefix;

        write.push(ts.const_(BODY, None, ts.index(ts.this(), ts.id(BODY_KEY))));
        write.extend(self.codec.write_value(ty, &format!("{BODY}.{VALUE}")));
        self.codec.reset_locals();

        let mut read = self.codec.read_value(ty, VALUE);

        read.push(ts.return_(Some(
            ts.new_(ts.path(class), vec![ts.object(vec![(VALUE, ts.id(VALUE))])]),
        )));

        let mut elements =
            expressions.statics(&plan.variant.tags, expressions.value_fields(), false);

        elements.extend([
            ts.computed_property(
                ts.id(BODY_KEY),
                ts.ty(&fields_type, Vec::new()),
                ModifierContainer::PROTECTED_READONLY,
            ),
            ts.constructor(
                vec![Parameter::new(BODY, ts.ty(&fields_type, Vec::new()))],
                vec![
                    ts.expression(ts.call(ts.super_(), Vec::new())),
                    ts.expression(ts.assign(ts.index(ts.this(), ts.id(BODY_KEY)), ts.id(BODY))),
                ],
            ),
            ts.getter(
                SPAN,
                VALUE,
                types.of(ty),
                vec![ts.return_(Some(ts.member(ts.index(ts.this(), ts.id(BODY_KEY)), VALUE)))],
            ),
            ts.method(
                SPAN,
                "write",
                vec![MemberBuilder::writer_parameter(ts)],
                ts.void(),
                write,
                ModifierContainer::OVERRIDE,
            ),
            ts.method(
                SPAN,
                "read",
                vec![MemberBuilder::reader_parameter(ts)],
                ts.ty(class, Vec::new()),
                read,
                ModifierContainer::STATIC,
            ),
        ]);

        let fields = ts.interface(
            FIELDS,
            vec![InterfaceProperty {
                name: VALUE.to_owned(),
                optional: false,
                ty: types.of(ty),
            }],
        );

        vec![
            ts.class(
                span,
                name,
                false,
                Some((ts.id(union), Vec::new())),
                elements,
            ),
            ts.namespace(name, vec![fields]),
        ]
    }

    fn enumeration(
        &self,
        docs: &DocumentationRef,
        tags: &TagList,
        name: &str,
        members: &[MemberPlan<'_>],
    ) -> Vec<Statement<'a>> {
        let ts = self.ts;
        let enumeration = ts.enumeration(
            ts.docs(docs),
            name,
            members
                .iter()
                .map(|plan| EnumMember {
                    span: ts.docs(&plan.member.docs),
                    name: plan.member.name.clone(),
                    value: plan.value,
                })
                .collect(),
        );
        let write = ts.exported_function(
            "write",
            vec![
                MemberBuilder::writer_parameter(ts),
                Parameter::new(VALUE, ts.ty(name, Vec::new())),
            ],
            ts.void(),
            members
                .iter()
                .filter_map(|plan| {
                    let guard = self.codec.version_guard(plan.since)?;

                    Some(ts.if_(
                        ts.binary(
                            ts.id(VALUE),
                            BinaryOperator::StrictEquality,
                            ts.path(&format!("{name}.{}", plan.member.name)),
                        ),
                        vec![guard],
                        None,
                    ))
                })
                .chain([ts.expression(ts.call_method(ts.id("writer"), "u32", vec![ts.id(VALUE)]))])
                .collect(),
        );
        let count =
            u32::try_from(members.len()).expect("an enumeration has fewer than 2^32 members");
        let positional = (0..).zip(members).all(|(index, plan)| plan.value == index);
        let is_unknown = if positional {
            ts.binary(
                ts.id(VALUE),
                BinaryOperator::GreaterEqualThan,
                ts.number_literal(count),
            )
        } else {
            ts.binary(
                ts.index(ts.id(name), ts.id(VALUE)),
                BinaryOperator::StrictEquality,
                ts.undefined_literal(),
            )
        };
        let unknown = ts.throw(ts.new_(
            ts.id(RANGE_ERROR),
            vec![ts.template(
                &["no member ", &format!(" of `{name}`")],
                vec![ts.id(VALUE)],
            )],
        ));
        let read = ts.exported_function(
            "read",
            vec![MemberBuilder::reader_parameter(ts)],
            ts.ty(name, Vec::new()),
            vec![
                ts.const_(
                    VALUE,
                    None,
                    ts.call_method(ts.id("reader"), "u32", Vec::new()),
                ),
                ts.if_(is_unknown, vec![unknown], None),
                ts.return_(Some(ts.id(VALUE))),
            ],
        );
        let expressions = self.tags();
        let member_tags = ts.array_literal(
            members
                .iter()
                .map(|plan| {
                    ts.object(vec![
                        ("name", ts.string_literal(&plan.member.name)),
                        ("value", ts.number_literal(plan.value)),
                        (TAGS, expressions.tags(&plan.member.tags)),
                    ])
                })
                .collect(),
        );
        let constants = [
            ts.exported_typed_const(
                SPAN,
                TAGS,
                Some(expressions.list(TAG)),
                expressions.tags(tags),
            ),
            ts.exported_typed_const(
                SPAN,
                MEMBERS,
                Some(expressions.list(MEMBER_TAGS)),
                member_tags,
            ),
        ];
        let mut namespace = vec![write, read];

        namespace.extend(constants);

        vec![enumeration, ts.namespace(name, namespace)]
    }
}
