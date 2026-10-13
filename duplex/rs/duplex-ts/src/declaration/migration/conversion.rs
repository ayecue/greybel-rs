use duplex_protocol::{
    Direction, Encoding, FieldPlan, FieldsPlan, PayloadPlan, Symbol, TypeContent,
};
use oxc_ast::ast::{BinaryOperator, Expression, Statement};
use oxc_span::SPAN;

use super::{ENTRY, ITEM, MigrationClass, OBJECT, RANGE_ERROR, VALUE};
use crate::ast::{ModifierContainer, Parameter, Ts};
use crate::types::TsTypeMapper;

impl<'a> MigrationClass<'_, '_, 'a> {
    pub(super) fn construct(
        ts: &Ts<'a>,
        class: &str,
        properties: Vec<(String, Expression<'a>)>,
    ) -> Expression<'a> {
        if properties.is_empty() {
            return ts.new_(ts.path(class), Vec::new());
        }

        let (names, values): (Vec<String>, Vec<Expression<'a>>) = properties.into_iter().unzip();
        let properties = names.iter().map(String::as_str).zip(values).collect();

        ts.new_(ts.path(class), vec![ts.object(properties)])
    }

    pub(super) fn convert_properties(
        &self,
        direction: Direction,
        from: &FieldsPlan<'_>,
        to: &FieldsPlan<'_>,
        source: &str,
    ) -> Vec<(String, Expression<'a>)> {
        let ts = self.ts;

        from.properties()
            .into_iter()
            .zip(to.properties())
            .map(|(from, to)| {
                let property = TsTypeMapper::property(to.field);
                let path = format!("{source}.{}", TsTypeMapper::property(from.field));
                let access = || ts.path(&path);

                (
                    property,
                    self.convert_property(direction, from, to, &access),
                )
            })
            .collect()
    }

    pub(super) fn convert_property(
        &self,
        direction: Direction,
        from: &FieldPlan<'_>,
        to: &FieldPlan<'_>,
        access: &dyn Fn() -> Expression<'a>,
    ) -> Expression<'a> {
        let ts = self.ts;

        if !from.field.optional || Self::is_plain(&from.ty) {
            return self.convert(direction, &from.ty, &to.ty, access);
        }

        ts.conditional(
            ts.binary(
                access(),
                BinaryOperator::StrictEquality,
                ts.undefined_literal(),
            ),
            ts.undefined_literal(),
            self.convert(direction, &from.ty, &to.ty, access),
        )
    }

    pub(super) fn is_plain(ty: &Encoding<'_>) -> bool {
        match ty {
            Encoding::List(item, _) | Encoding::Record(item, _) | Encoding::Nullable(item) => {
                Self::is_plain(item)
            }
            Encoding::Struct(_) | Encoding::Union(_) | Encoding::Enumeration(_) => false,
            _ => true,
        }
    }

    pub(super) fn convert(
        &self,
        direction: Direction,
        from: &Encoding<'_>,
        to: &Encoding<'_>,
        value: &dyn Fn() -> Expression<'a>,
    ) -> Expression<'a> {
        let ts = self.ts;

        if Self::is_plain(from) {
            return value();
        }

        match (from, to) {
            (Encoding::List(from, _), Encoding::List(to, _)) => ts.call_method(
                value(),
                "map",
                vec![ts.arrow(ITEM, self.convert(direction, from, to, &|| ts.id(ITEM)))],
            ),
            (Encoding::Record(from, _), Encoding::Record(to, _)) => {
                let entry = |index: u32| ts.index(ts.id(ENTRY), ts.number_literal(index));
                let entry_array = ts.array_literal(vec![
                    entry(0),
                    self.convert(direction, from, to, &|| entry(1)),
                ]);

                ts.call_method(
                    ts.id(OBJECT),
                    "fromEntries",
                    vec![ts.call_method(
                        ts.call_method(ts.id(OBJECT), "entries", vec![value()]),
                        "map",
                        vec![ts.arrow(ENTRY, entry_array)],
                    )],
                )
            }
            (Encoding::Nullable(from), Encoding::Nullable(to)) => ts.conditional(
                ts.binary(value(), BinaryOperator::StrictEquality, ts.null_literal()),
                ts.null_literal(),
                self.convert(direction, from, to, value),
            ),
            (from, to) => {
                let (Some(from), Some(to)) = (from.declared_name(), to.declared_name()) else {
                    return value();
                };
                let method = self.conversion(direction, from, to);

                ts.call_method(ts.id(&self.name), &method, vec![value()])
            }
        }
    }

    pub(super) fn conversion(&self, direction: Direction, from: &str, to: &str) -> String {
        let ts = self.ts;
        let prefix = match direction {
            Direction::Up => "up",
            Direction::Down => "down",
        };
        let method = format!("{prefix}{}", Symbol::Declared(from).pascal());
        let key = (direction, from.to_owned());

        if self.conversions.borrow().contains_key(&key) {
            return method;
        }

        let from_newer = direction.from_newer();
        let (Some(from_plan), Some(to_plan)) = (
            self.plan.declaration(from_newer, from),
            self.plan.declaration(!from_newer, to),
        ) else {
            return method;
        };
        let placeholder = ts.method(
            SPAN,
            &method,
            Vec::new(),
            ts.void(),
            Vec::new(),
            ModifierContainer::STATIC,
        );

        self.conversions
            .borrow_mut()
            .insert(key.clone(), placeholder);

        let from_types = self.types(from_newer);
        let to_types = self.types(!from_newer);
        let from_class = from_types.declared_name(from);
        let to_class = to_types.declared_name(to);
        let body = match (&from_plan.content, &to_plan.content) {
            (TypeContent::Struct(from_fields), TypeContent::Struct(to_fields)) => {
                let properties = self.convert_properties(direction, from_fields, to_fields, VALUE);

                vec![ts.return_(Some(Self::construct(ts, &to_class, properties)))]
            }
            (TypeContent::Union(from_variants), TypeContent::Union(to_variants)) => {
                let mut body: Vec<Statement<'a>> = from_variants
                    .iter()
                    .zip(to_variants)
                    .map(|(from, to)| {
                        let from_variant = format!("{from_class}.{}", from.variant.name);
                        let to_variant = format!("{to_class}.{}", to.variant.name);
                        let variant_expr = match (&from.payload, &to.payload) {
                            (PayloadPlan::Value(from_value), PayloadPlan::Value(to_value)) => {
                                let value_expr =
                                    self.convert(direction, from_value, to_value, &|| {
                                        ts.path(&format!("{VALUE}.{VALUE}"))
                                    });

                                Self::construct(
                                    ts,
                                    &to_variant,
                                    vec![(VALUE.to_owned(), value_expr)],
                                )
                            }
                            (PayloadPlan::Fields(from_fields), PayloadPlan::Fields(to_fields)) => {
                                let properties = self.convert_properties(
                                    direction,
                                    from_fields,
                                    to_fields,
                                    VALUE,
                                );

                                Self::construct(ts, &to_variant, properties)
                            }
                            _ => Self::construct(ts, &to_variant, Vec::new()),
                        };

                        ts.if_(
                            ts.binary(
                                ts.id(VALUE),
                                BinaryOperator::Instanceof,
                                ts.path(&from_variant),
                            ),
                            vec![ts.return_(Some(variant_expr))],
                            None,
                        )
                    })
                    .collect();

                body.push(ts.throw(ts.new_(
                    ts.id(RANGE_ERROR),
                    vec![ts.string_literal(&format!("no variant of `{from}`"))],
                )));

                body
            }
            (TypeContent::Enumeration(_), TypeContent::Enumeration(_)) => {
                vec![ts.return_(Some(ts.as_(
                    ts.as_(ts.id(VALUE), ts.unknown()),
                    ts.ty(&to_class, Vec::new()),
                )))]
            }
            _ => Vec::new(),
        };
        let element = ts.method(
            SPAN,
            &method,
            vec![Parameter::new(VALUE, ts.ty(&from_class, Vec::new()))],
            ts.ty(&to_class, Vec::new()),
            body,
            ModifierContainer::STATIC,
        );

        self.conversions.borrow_mut().insert(key, element);

        method
    }
}
