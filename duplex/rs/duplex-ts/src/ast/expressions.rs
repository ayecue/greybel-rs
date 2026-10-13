use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionBody, AssignmentOperator, AssignmentTarget,
    BigintBase, BinaryOperator, BindingPattern, Expression, FormalParameter, FormalParameterKind,
    FormalParameters, IdentifierName, LogicalOperator, NumberBase, ObjectPropertyKind, PropertyKey,
    PropertyKind, SimpleAssignmentTarget, TSType, TemplateElement, TemplateElementValue,
    UpdateOperator,
};
use oxc_span::SPAN;

use super::Ts;

const CONST: &str = "const";
const THIS: &str = "this";
const UNDEFINED: &str = "undefined";
const PATH_SEPARATOR: char = '.';

impl<'a> Ts<'a> {
    pub(crate) fn id(&self, name: &str) -> Expression<'a> {
        Expression::new_identifier(SPAN, self.text(name), self)
    }

    pub(crate) fn this(&self) -> Expression<'a> {
        Expression::new_this_expression(SPAN, self)
    }

    pub(crate) fn super_(&self) -> Expression<'a> {
        Expression::new_super(SPAN, self)
    }

    pub(crate) fn number_literal(&self, value: impl Into<f64>) -> Expression<'a> {
        Expression::new_numeric_literal(SPAN, value.into(), None, NumberBase::Decimal, self)
    }

    pub(crate) fn bigint_literal(&self, value: i128) -> Expression<'a> {
        Expression::new_big_int_literal(
            SPAN,
            self.text(&value.to_string()),
            None,
            BigintBase::Decimal,
            self,
        )
    }

    pub(crate) fn string_literal(&self, value: &str) -> Expression<'a> {
        Expression::new_string_literal(SPAN, self.text(value), None, self)
    }

    pub(crate) fn boolean_literal(&self, value: bool) -> Expression<'a> {
        Expression::new_boolean_literal(SPAN, value, self)
    }

    pub(crate) fn null_literal(&self) -> Expression<'a> {
        Expression::new_null_literal(SPAN, self)
    }

    pub(crate) fn undefined_literal(&self) -> Expression<'a> {
        self.id(UNDEFINED)
    }

    pub(crate) fn arrow(&self, parameter: &str, body: Expression<'a>) -> Expression<'a> {
        let parameter = FormalParameter::new(
            SPAN,
            self.vec([]),
            BindingPattern::new_binding_identifier(SPAN, self.text(parameter), self),
            None,
            None,
            false,
            None,
            false,
            false,
            self,
        );
        let parameters = FormalParameters::boxed(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            self.vec([parameter]),
            None,
            self,
        );

        Expression::new_arrow_function_expression(
            SPAN,
            false,
            None,
            parameters,
            None,
            ArrowFunctionBody::from(body),
            self,
        )
    }

    pub(crate) fn member(&self, object: Expression<'a>, property: &str) -> Expression<'a> {
        Expression::new_static_member_expression(
            SPAN,
            object,
            IdentifierName::new(SPAN, self.text(property), self),
            false,
            self,
        )
    }

    pub(crate) fn index(&self, object: Expression<'a>, index: Expression<'a>) -> Expression<'a> {
        Expression::new_computed_member_expression(SPAN, object, index, false, self)
    }

    pub(crate) fn non_null(&self, expression: Expression<'a>) -> Expression<'a> {
        Expression::new_ts_non_null_expression(SPAN, expression, self)
    }

    pub(crate) fn path(&self, path: &str) -> Expression<'a> {
        let mut segments = path.split(PATH_SEPARATOR);
        let first = match segments.next() {
            Some(THIS) => self.this(),
            first => self.id(first.unwrap_or(path)),
        };

        segments.fold(first, |object, segment| self.member(object, segment))
    }

    pub(crate) fn call(
        &self,
        callee: Expression<'a>,
        arguments: Vec<Expression<'a>>,
    ) -> Expression<'a> {
        Expression::new_call_expression(
            SPAN,
            callee,
            None,
            self.call_arguments(arguments),
            false,
            self,
        )
    }

    pub(crate) fn call_method(
        &self,
        object: Expression<'a>,
        method: &str,
        arguments: Vec<Expression<'a>>,
    ) -> Expression<'a> {
        let callee = self.member(object, method);

        self.call(callee, arguments)
    }

    pub(crate) fn new_(
        &self,
        class: Expression<'a>,
        arguments: Vec<Expression<'a>>,
    ) -> Expression<'a> {
        Expression::new_new_expression(SPAN, class, None, self.call_arguments(arguments), self)
    }

    fn call_arguments(
        &self,
        arguments: Vec<Expression<'a>>,
    ) -> oxc_allocator::ArenaVec<'a, Argument<'a>> {
        self.vec(arguments.into_iter().map(Argument::from))
    }

    pub(crate) fn binary(
        &self,
        left: Expression<'a>,
        operator: BinaryOperator,
        right: Expression<'a>,
    ) -> Expression<'a> {
        Expression::new_binary_expression(SPAN, left, operator, right, self)
    }

    pub(crate) fn and(&self, left: Expression<'a>, right: Expression<'a>) -> Expression<'a> {
        Expression::new_logical_expression(SPAN, left, LogicalOperator::And, right, self)
    }

    pub(crate) fn coalesce(&self, left: Expression<'a>, right: Expression<'a>) -> Expression<'a> {
        Expression::new_logical_expression(SPAN, left, LogicalOperator::Coalesce, right, self)
    }

    pub(crate) fn conditional(
        &self,
        test: Expression<'a>,
        consequent: Expression<'a>,
        alternate: Expression<'a>,
    ) -> Expression<'a> {
        Expression::new_conditional_expression(SPAN, test, consequent, alternate, self)
    }

    pub(crate) fn wrap_in_parens(&self, expression: Expression<'a>) -> Expression<'a> {
        Expression::new_parenthesized_expression(SPAN, expression, self)
    }

    pub(crate) fn assign(&self, target: Expression<'a>, value: Expression<'a>) -> Expression<'a> {
        let target = match target {
            Expression::Identifier(identifier) => {
                AssignmentTarget::new_assignment_target_identifier(SPAN, identifier.name, self)
            }
            Expression::StaticMemberExpression(member) => {
                let member = member.unbox();

                AssignmentTarget::new_static_member_expression(
                    SPAN,
                    member.object,
                    member.property,
                    false,
                    self,
                )
            }
            Expression::ComputedMemberExpression(member) => {
                let member = member.unbox();

                AssignmentTarget::new_computed_member_expression(
                    SPAN,
                    member.object,
                    member.expression,
                    false,
                    self,
                )
            }
            _ => unreachable!("assignments are to names or properties"),
        };

        Expression::new_assignment_expression(SPAN, AssignmentOperator::Assign, target, value, self)
    }

    pub(crate) fn decrement(&self, name: &str) -> Expression<'a> {
        Expression::new_update_expression(
            SPAN,
            UpdateOperator::Decrement,
            false,
            SimpleAssignmentTarget::new_assignment_target_identifier(SPAN, self.text(name), self),
            self,
        )
    }

    pub(crate) fn as_(&self, expression: Expression<'a>, ty: TSType<'a>) -> Expression<'a> {
        Expression::new_ts_as_expression(SPAN, expression, ty, self)
    }

    pub(crate) fn as_const(&self, expression: Expression<'a>) -> Expression<'a> {
        let ty = self.ty(CONST, Vec::new());

        self.as_(expression, ty)
    }

    pub(crate) fn object(&self, properties: Vec<(&str, Expression<'a>)>) -> Expression<'a> {
        let properties = properties.into_iter().map(|(key, value)| {
            let shorthand =
                matches!(&value, Expression::Identifier(identifier) if identifier.name == key);

            ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::new_static_identifier(SPAN, self.text(key), self),
                value,
                false,
                shorthand,
                false,
                self,
            )
        });

        Expression::new_object_expression(SPAN, self.vec(properties), self)
    }

    pub(crate) fn spread(
        &self,
        base: Expression<'a>,
        properties: Vec<(&str, Expression<'a>)>,
    ) -> Expression<'a> {
        let Expression::ObjectExpression(mut object) = self.object(properties) else {
            unreachable!("an object");
        };

        object
            .properties
            .insert(0, ObjectPropertyKind::new_spread_property(SPAN, base, self));

        Expression::ObjectExpression(object)
    }

    pub(crate) fn array_literal(&self, items: Vec<Expression<'a>>) -> Expression<'a> {
        Expression::new_array_expression(
            SPAN,
            self.vec(items.into_iter().map(ArrayExpressionElement::from)),
            self,
        )
    }

    pub(crate) fn template(
        &self,
        texts: &[&str],
        expressions: Vec<Expression<'a>>,
    ) -> Expression<'a> {
        let last = texts.len().saturating_sub(1);
        let quasis = texts.iter().enumerate().map(|(index, text)| {
            let raw = text
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace("${", "\\${");

            TemplateElement::new(
                SPAN,
                TemplateElementValue {
                    raw: self.text(&raw).into(),
                    cooked: Some(self.text(text).into()),
                },
                index == last,
                self,
            )
        });

        Expression::new_template_literal(SPAN, self.vec(quasis), self.vec(expressions), self)
    }
}
