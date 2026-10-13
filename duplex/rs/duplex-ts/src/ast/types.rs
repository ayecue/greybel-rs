use oxc_allocator::ArenaBox;
use oxc_ast::ast::{
    IdentifierName, TSType, TSTypeAnnotation, TSTypeName, TSTypeParameterInstantiation,
    TSTypeQueryExprName,
};
use oxc_span::SPAN;

use super::Ts;

const PATH_SEPARATOR: char = '.';

impl<'a> Ts<'a> {
    pub(crate) fn ty(&self, path: &str, arguments: Vec<TSType<'a>>) -> TSType<'a> {
        TSType::new_ts_type_reference(SPAN, self.type_name(path), self.arguments(arguments), self)
    }

    pub(crate) fn type_of(&self, path: &str) -> TSType<'a> {
        let mut segments = path.rsplitn(2, PATH_SEPARATOR);
        let last = segments.next().unwrap_or(path);
        let name = match segments.next() {
            Some(left) => TSTypeQueryExprName::new_qualified_name(
                SPAN,
                self.type_name(left),
                IdentifierName::new(SPAN, self.text(last), self),
                self,
            ),
            None => TSTypeQueryExprName::new_identifier_reference(SPAN, self.text(path), self),
        };

        TSType::new_ts_type_query(SPAN, name, None, self)
    }

    fn type_name(&self, path: &str) -> TSTypeName<'a> {
        let mut segments = path.split(PATH_SEPARATOR);
        let first = segments.next().unwrap_or(path);
        let mut name = TSTypeName::new_identifier_reference(SPAN, self.text(first), self);

        for segment in segments {
            name = TSTypeName::new_qualified_name(
                SPAN,
                name,
                IdentifierName::new(SPAN, self.text(segment), self),
                self,
            );
        }

        name
    }

    pub(crate) fn arguments(
        &self,
        arguments: Vec<TSType<'a>>,
    ) -> Option<ArenaBox<'a, TSTypeParameterInstantiation<'a>>> {
        (!arguments.is_empty())
            .then(|| TSTypeParameterInstantiation::boxed(SPAN, self.vec(arguments), self))
    }

    pub(crate) fn boolean(&self) -> TSType<'a> {
        TSType::new_ts_boolean_keyword(SPAN, self)
    }

    pub(crate) fn number(&self) -> TSType<'a> {
        TSType::new_ts_number_keyword(SPAN, self)
    }

    pub(crate) fn bigint(&self) -> TSType<'a> {
        TSType::new_ts_big_int_keyword(SPAN, self)
    }

    pub(crate) fn string(&self) -> TSType<'a> {
        TSType::new_ts_string_keyword(SPAN, self)
    }

    pub(crate) fn void(&self) -> TSType<'a> {
        TSType::new_ts_void_keyword(SPAN, self)
    }

    pub(crate) fn unknown(&self) -> TSType<'a> {
        TSType::new_ts_unknown_keyword(SPAN, self)
    }

    pub(crate) fn null(&self) -> TSType<'a> {
        TSType::new_ts_null_keyword(SPAN, self)
    }

    pub(crate) fn undefined_type(&self) -> TSType<'a> {
        TSType::new_ts_undefined_keyword(SPAN, self)
    }

    pub(crate) fn array(&self, item: TSType<'a>) -> TSType<'a> {
        TSType::new_ts_array_type(SPAN, item, self)
    }

    pub(crate) fn union(&self, types: Vec<TSType<'a>>) -> TSType<'a> {
        TSType::new_ts_union_type(SPAN, self.vec(types), self)
    }

    pub(crate) fn annotation(&self, ty: TSType<'a>) -> ArenaBox<'a, TSTypeAnnotation<'a>> {
        TSTypeAnnotation::boxed(SPAN, ty, self)
    }
}
