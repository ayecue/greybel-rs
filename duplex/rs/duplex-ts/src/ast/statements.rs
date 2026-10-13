use oxc_ast::ast::{
    BinaryOperator, BindingPattern, Expression, ForStatementInit, ForStatementLeft, Statement,
    SwitchCase, TSType, VariableDeclarationKind, VariableDeclarator,
};
use oxc_span::SPAN;

use super::Ts;

impl<'a> Ts<'a> {
    pub(crate) fn expression(&self, expression: Expression<'a>) -> Statement<'a> {
        Statement::new_expression_statement(SPAN, expression, self)
    }

    pub(crate) fn return_(&self, value: Option<Expression<'a>>) -> Statement<'a> {
        Statement::new_return_statement(SPAN, value, self)
    }

    pub(crate) fn throw(&self, error: Expression<'a>) -> Statement<'a> {
        Statement::new_throw_statement(SPAN, error, self)
    }

    pub(crate) fn block(&self, body: Vec<Statement<'a>>) -> Statement<'a> {
        Statement::new_block_statement(SPAN, self.vec(body), self)
    }

    fn declarator(
        &self,
        name: &str,
        ty: Option<TSType<'a>>,
        init: Option<Expression<'a>>,
    ) -> VariableDeclarator<'a> {
        VariableDeclarator::new(
            SPAN,
            BindingPattern::new_binding_identifier(SPAN, self.text(name), self),
            ty.map(|ty| self.annotation(ty)),
            init,
            false,
            self,
        )
    }

    pub(crate) fn const_(
        &self,
        name: &str,
        ty: Option<TSType<'a>>,
        value: Expression<'a>,
    ) -> Statement<'a> {
        self.variable(VariableDeclarationKind::Const, name, ty, Some(value))
    }

    pub(crate) fn let_(&self, name: &str, ty: TSType<'a>, value: Expression<'a>) -> Statement<'a> {
        self.variable(VariableDeclarationKind::Let, name, Some(ty), Some(value))
    }

    fn variable(
        &self,
        kind: VariableDeclarationKind,
        name: &str,
        ty: Option<TSType<'a>>,
        value: Option<Expression<'a>>,
    ) -> Statement<'a> {
        Statement::new_variable_declaration(
            SPAN,
            kind,
            self.vec([self.declarator(name, ty, value)]),
            false,
            self,
        )
    }

    pub(crate) fn if_(
        &self,
        test: Expression<'a>,
        consequent: Vec<Statement<'a>>,
        alternate: Option<Vec<Statement<'a>>>,
    ) -> Statement<'a> {
        Statement::new_if_statement(
            SPAN,
            test,
            self.block(consequent),
            alternate.map(|alternate| self.block(alternate)),
            self,
        )
    }

    pub(crate) fn for_of(
        &self,
        item: &str,
        items: Expression<'a>,
        body: Vec<Statement<'a>>,
    ) -> Statement<'a> {
        let left = ForStatementLeft::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            self.vec([self.declarator(item, None, None)]),
            false,
            self,
        );

        Statement::new_for_of_statement(SPAN, false, left, items, self.block(body), self)
    }

    pub(crate) fn repeat(
        &self,
        count: &str,
        start: Expression<'a>,
        body: Vec<Statement<'a>>,
    ) -> Statement<'a> {
        let init = ForStatementInit::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Let,
            self.vec([self.declarator(count, None, Some(start))]),
            false,
            self,
        );
        let test = self.binary(
            self.id(count),
            BinaryOperator::GreaterThan,
            self.number_literal(0),
        );

        Statement::new_for_statement(
            SPAN,
            Some(init),
            Some(test),
            Some(self.decrement(count)),
            self.block(body),
            self,
        )
    }

    pub(crate) fn switch(
        &self,
        value: Expression<'a>,
        cases: Vec<(Expression<'a>, Vec<Statement<'a>>)>,
        default: Vec<Statement<'a>>,
    ) -> Statement<'a> {
        let cases = cases
            .into_iter()
            .map(|(test, body)| SwitchCase::new(SPAN, Some(test), self.vec(body), self))
            .chain([SwitchCase::new(SPAN, None, self.vec(default), self)]);

        Statement::new_switch_statement(SPAN, value, self.vec(cases), self)
    }
}
