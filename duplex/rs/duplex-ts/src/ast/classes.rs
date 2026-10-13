use oxc_allocator::ArenaBox;
use oxc_ast::ast::{
    BindingIdentifier, BindingPattern, ClassElement, Expression, FormalParameter,
    FormalParameterKind, FormalParameters, Function, FunctionBody, FunctionType,
    MethodDefinitionKind, MethodDefinitionType, PropertyDefinitionType, PropertyKey, Statement,
    TSAccessibility, TSType, TSTypeParameter, TSTypeParameterDeclaration,
};
use oxc_span::{SPAN, Span};

use super::Ts;

const CONSTRUCTOR: &str = "constructor";

/// Type parameter `name extends constraint`.
pub(crate) struct TypeParameter<'a> {
    pub(crate) name: &'static str,
    pub(crate) constraint: TSType<'a>,
}

pub(crate) struct Parameter<'a> {
    pub(crate) name: &'static str,
    pub(crate) ty: TSType<'a>,
    pub(crate) optional: bool,
}

impl<'a> Parameter<'a> {
    pub(crate) fn new(name: &'static str, ty: TSType<'a>) -> Self {
        Self {
            name,
            ty,
            optional: false,
        }
    }

    pub(crate) fn optional(name: &'static str, ty: TSType<'a>) -> Self {
        Self {
            name,
            ty,
            optional: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one TypeScript modifier a member may carry alongside the others"
)]
pub(crate) struct ModifierContainer {
    pub(crate) is_static: bool,
    pub(crate) readonly: bool,
    pub(crate) protected: bool,
    pub(crate) is_override: bool,
}

impl ModifierContainer {
    pub(crate) const NONE: Self = Self {
        is_static: false,
        readonly: false,
        protected: false,
        is_override: false,
    };
    pub(crate) const OVERRIDE: Self = Self {
        is_override: true,
        ..Self::NONE
    };
    pub(crate) const PROTECTED_READONLY: Self = Self {
        protected: true,
        readonly: true,
        ..Self::NONE
    };
    pub(crate) const STATIC: Self = Self {
        is_static: true,
        ..Self::NONE
    };
    pub(crate) const STATIC_READONLY: Self = Self {
        is_static: true,
        readonly: true,
        ..Self::NONE
    };

    pub(crate) fn with_override(self, is_override: bool) -> Self {
        Self {
            is_override,
            ..self
        }
    }

    fn accessibility(self) -> Option<TSAccessibility> {
        self.protected.then_some(TSAccessibility::Protected)
    }
}

impl<'a> Ts<'a> {
    fn key(&self, name: &str) -> PropertyKey<'a> {
        PropertyKey::new_static_identifier(SPAN, self.text(name), self)
    }

    fn parameters(&self, parameters: Vec<Parameter<'a>>) -> ArenaBox<'a, FormalParameters<'a>> {
        let items = parameters.into_iter().map(|parameter| {
            FormalParameter::new(
                SPAN,
                self.vec([]),
                BindingPattern::new_binding_identifier(SPAN, self.text(parameter.name), self),
                Some(self.annotation(parameter.ty)),
                None,
                parameter.optional,
                None,
                false,
                false,
                self,
            )
        });

        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::FormalParameter,
            self.vec(items),
            None,
            self,
        )
    }

    fn type_parameters(
        &self,
        type_parameters: Vec<TypeParameter<'a>>,
    ) -> Option<ArenaBox<'a, TSTypeParameterDeclaration<'a>>> {
        if type_parameters.is_empty() {
            return None;
        }

        let items = type_parameters.into_iter().map(|parameter| {
            TSTypeParameter::new(
                SPAN,
                BindingIdentifier::new(SPAN, self.text(parameter.name), self),
                Some(parameter.constraint),
                None,
                false,
                false,
                false,
                self,
            )
        });

        Some(TSTypeParameterDeclaration::boxed(
            SPAN,
            self.vec(items),
            self,
        ))
    }

    pub(crate) fn function(
        &self,
        kind: FunctionType,
        name: Option<&str>,
        type_parameters: Vec<TypeParameter<'a>>,
        parameters: Vec<Parameter<'a>>,
        returns: Option<TSType<'a>>,
        body: Option<Vec<Statement<'a>>>,
    ) -> ArenaBox<'a, Function<'a>> {
        Function::boxed(
            SPAN,
            kind,
            name.map(|name| BindingIdentifier::new(SPAN, self.text(name), self)),
            false,
            false,
            false,
            self.type_parameters(type_parameters),
            None,
            self.parameters(parameters),
            returns.map(|returns| self.annotation(returns)),
            body.map(|body| FunctionBody::boxed(SPAN, self.vec([]), self.vec(body), self)),
            self,
        )
    }

    pub(crate) fn property(
        &self,
        span: Span,
        name: &str,
        ty: Option<TSType<'a>>,
        value: Option<Expression<'a>>,
        modifiers: ModifierContainer,
    ) -> ClassElement<'a> {
        ClassElement::new_property_definition(
            span,
            PropertyDefinitionType::PropertyDefinition,
            self.vec([]),
            self.key(name),
            ty.map(|ty| self.annotation(ty)),
            value,
            false,
            modifiers.is_static,
            false,
            modifiers.is_override,
            false,
            false,
            modifiers.readonly,
            modifiers.accessibility(),
            self,
        )
    }

    /// Creates computed class property `[key]: ty`.
    pub(crate) fn computed_property(
        &self,
        key: Expression<'a>,
        ty: TSType<'a>,
        modifiers: ModifierContainer,
    ) -> ClassElement<'a> {
        ClassElement::new_property_definition(
            SPAN,
            PropertyDefinitionType::PropertyDefinition,
            self.vec([]),
            PropertyKey::from(key),
            Some(self.annotation(ty)),
            None,
            true,
            modifiers.is_static,
            false,
            modifiers.is_override,
            false,
            false,
            modifiers.readonly,
            modifiers.accessibility(),
            self,
        )
    }

    pub(crate) fn constructor(
        &self,
        parameters: Vec<Parameter<'a>>,
        body: Vec<Statement<'a>>,
    ) -> ClassElement<'a> {
        self.method_definition(
            SPAN,
            CONSTRUCTOR,
            MethodDefinitionKind::Constructor,
            Vec::new(),
            parameters,
            None,
            Some(body),
            ModifierContainer::NONE,
        )
    }

    pub(crate) fn method(
        &self,
        span: Span,
        name: &str,
        parameters: Vec<Parameter<'a>>,
        returns: TSType<'a>,
        body: Vec<Statement<'a>>,
        modifiers: ModifierContainer,
    ) -> ClassElement<'a> {
        self.method_definition(
            span,
            name,
            MethodDefinitionKind::Method,
            Vec::new(),
            parameters,
            Some(returns),
            Some(body),
            modifiers,
        )
    }

    /// Creates a method with generic `type_parameters`.
    pub(crate) fn generic_method(
        &self,
        name: &str,
        type_parameters: Vec<TypeParameter<'a>>,
        parameters: Vec<Parameter<'a>>,
        returns: TSType<'a>,
        body: Vec<Statement<'a>>,
        modifiers: ModifierContainer,
    ) -> ClassElement<'a> {
        self.method_definition(
            SPAN,
            name,
            MethodDefinitionKind::Method,
            type_parameters,
            parameters,
            Some(returns),
            Some(body),
            modifiers,
        )
    }

    pub(crate) fn abstract_method(
        &self,
        name: &str,
        parameters: Vec<Parameter<'a>>,
        returns: TSType<'a>,
    ) -> ClassElement<'a> {
        self.method_definition(
            SPAN,
            name,
            MethodDefinitionKind::Method,
            Vec::new(),
            parameters,
            Some(returns),
            None,
            ModifierContainer::NONE,
        )
    }

    pub(crate) fn getter(
        &self,
        span: Span,
        name: &str,
        returns: TSType<'a>,
        body: Vec<Statement<'a>>,
    ) -> ClassElement<'a> {
        self.method_definition(
            span,
            name,
            MethodDefinitionKind::Get,
            Vec::new(),
            Vec::new(),
            Some(returns),
            Some(body),
            ModifierContainer::NONE,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the parts of a method definition"
    )]
    fn method_definition(
        &self,
        span: Span,
        name: &str,
        kind: MethodDefinitionKind,
        type_parameters: Vec<TypeParameter<'a>>,
        parameters: Vec<Parameter<'a>>,
        returns: Option<TSType<'a>>,
        body: Option<Vec<Statement<'a>>>,
        modifiers: ModifierContainer,
    ) -> ClassElement<'a> {
        let (definition, function) = if body.is_some() {
            (
                MethodDefinitionType::MethodDefinition,
                FunctionType::FunctionExpression,
            )
        } else {
            (
                MethodDefinitionType::TSAbstractMethodDefinition,
                FunctionType::TSEmptyBodyFunctionExpression,
            )
        };

        ClassElement::new_method_definition(
            span,
            definition,
            self.vec([]),
            self.key(name),
            self.function(function, None, type_parameters, parameters, returns, body),
            kind,
            false,
            modifiers.is_static,
            modifiers.is_override,
            false,
            modifiers.accessibility(),
            self,
        )
    }
}
