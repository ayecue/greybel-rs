use oxc_ast::ast::{
    BindingIdentifier, BindingPattern, ClassBody, ClassElement, ClassHeritage, ClassType,
    Declaration, Expression, FunctionType, ImportDeclarationSpecifier, ImportOrExportKind,
    ModuleExportName, PropertyKey, Statement, StringLiteral, TSEnumBody, TSEnumMember,
    TSEnumMemberName, TSInterfaceBody, TSNamespaceDeclarationBody, TSNamespaceDeclarationKind,
    TSSignature, TSType, VariableDeclarationKind, VariableDeclarator,
};
use oxc_span::{SPAN, Span};

use super::{Parameter, Ts};

pub(crate) struct EnumMember {
    pub(crate) span: Span,
    pub(crate) name: String,
    pub(crate) value: u32,
}

pub(crate) struct InterfaceProperty<'a> {
    pub(crate) name: String,
    pub(crate) optional: bool,
    pub(crate) ty: TSType<'a>,
}

pub(crate) struct Import {
    pub(crate) name: String,
    pub(crate) type_only: bool,
}

impl Import {
    pub(crate) fn value(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_only: false,
        }
    }

    pub(crate) fn type_only(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_only: true,
        }
    }
}

impl<'a> Ts<'a> {
    fn binding(&self, name: &str) -> BindingIdentifier<'a> {
        BindingIdentifier::new(SPAN, self.text(name), self)
    }

    fn export(&self, span: Span, declaration: Declaration<'a>) -> Statement<'a> {
        Statement::new_export_declaration(span, declaration, self)
    }

    /// Creates an import from `source`. Uses `import type` when every binding is type-only, so no runtime import is
    /// emitted.
    pub(crate) fn import(&self, names: Vec<Import>, source: &str) -> Statement<'a> {
        let type_only = !names.is_empty() && names.iter().all(|import| import.type_only);
        let specifiers = names.into_iter().map(|import| {
            ImportDeclarationSpecifier::new_import_specifier(
                SPAN,
                ModuleExportName::new_identifier_name(SPAN, self.text(&import.name), self),
                self.binding(&import.name),
                if import.type_only && !type_only {
                    ImportOrExportKind::Type
                } else {
                    ImportOrExportKind::Value
                },
                self,
            )
        });

        Statement::new_import_declaration(
            SPAN,
            Some(self.vec(specifiers)),
            StringLiteral::new(SPAN, self.text(source), None, self),
            None,
            None,
            if type_only {
                ImportOrExportKind::Type
            } else {
                ImportOrExportKind::Value
            },
            self,
        )
    }

    pub(crate) fn import_namespace(&self, namespace: &str, source: &str) -> Statement<'a> {
        let specifier = ImportDeclarationSpecifier::new_import_namespace_specifier(
            SPAN,
            self.binding(namespace),
            self,
        );

        Statement::new_import_declaration(
            SPAN,
            Some(self.vec([specifier])),
            StringLiteral::new(SPAN, self.text(source), None, self),
            None,
            None,
            ImportOrExportKind::Value,
            self,
        )
    }

    pub(crate) fn export_all(&self, source: &str) -> Statement<'a> {
        Statement::new_export_all_declaration(
            SPAN,
            None,
            StringLiteral::new(SPAN, self.text(source), None, self),
            None,
            ImportOrExportKind::Value,
            self,
        )
    }

    pub(crate) fn class(
        &self,
        span: Span,
        name: &str,
        is_abstract: bool,
        base: Option<(Expression<'a>, Vec<TSType<'a>>)>,
        members: Vec<ClassElement<'a>>,
    ) -> Statement<'a> {
        let heritage = base
            .map(|(class, arguments)| ClassHeritage::new(class, self.arguments(arguments), self));
        let declaration = Declaration::new_class_declaration(
            SPAN,
            ClassType::ClassDeclaration,
            self.vec([]),
            Some(self.binding(name)),
            None,
            heritage,
            self.vec([]),
            ClassBody::boxed(SPAN, self.vec(members), self),
            is_abstract,
            false,
            self,
        );

        self.export(span, declaration)
    }

    pub(crate) fn enumeration(
        &self,
        span: Span,
        name: &str,
        members: Vec<EnumMember>,
    ) -> Statement<'a> {
        let members = members.into_iter().map(|member| {
            TSEnumMember::new(
                member.span,
                TSEnumMemberName::new_identifier(SPAN, self.text(&member.name), self),
                Some(self.number_literal(member.value)),
                self,
            )
        });
        let declaration = Declaration::new_ts_enum_declaration(
            SPAN,
            self.binding(name),
            TSEnumBody::new(SPAN, self.vec(members), self),
            false,
            false,
            self,
        );

        self.export(span, declaration)
    }

    pub(crate) fn namespace(&self, name: &str, body: Vec<Statement<'a>>) -> Statement<'a> {
        let declaration = Declaration::new_ts_namespace_declaration(
            SPAN,
            self.binding(name),
            TSNamespaceDeclarationBody::new_ts_module_block(
                SPAN,
                self.vec([]),
                self.vec(body),
                self,
            ),
            TSNamespaceDeclarationKind::Namespace,
            false,
            self,
        );

        self.export(SPAN, declaration)
    }

    pub(crate) fn interface(
        &self,
        name: &str,
        properties: Vec<InterfaceProperty<'a>>,
    ) -> Statement<'a> {
        let signatures = properties.into_iter().map(|property| {
            TSSignature::new_ts_property_signature(
                SPAN,
                false,
                property.optional,
                false,
                PropertyKey::new_static_identifier(SPAN, self.text(&property.name), self),
                Some(self.annotation(property.ty)),
                self,
            )
        });
        let declaration = Declaration::new_ts_interface_declaration(
            SPAN,
            self.binding(name),
            None,
            self.vec([]),
            TSInterfaceBody::boxed(SPAN, self.vec(signatures), self),
            false,
            self,
        );

        self.export(SPAN, declaration)
    }

    pub(crate) fn exported_function(
        &self,
        name: &str,
        parameters: Vec<Parameter<'a>>,
        returns: TSType<'a>,
        body: Vec<Statement<'a>>,
    ) -> Statement<'a> {
        let function = self.function(
            FunctionType::FunctionDeclaration,
            Some(name),
            Vec::new(),
            parameters,
            Some(returns),
            Some(body),
        );

        self.export(SPAN, Declaration::FunctionDeclaration(function))
    }

    pub(crate) fn exported_const(
        &self,
        span: Span,
        name: &str,
        value: Expression<'a>,
    ) -> Statement<'a> {
        self.exported_typed_const(span, name, None, value)
    }

    pub(crate) fn exported_typed_const(
        &self,
        span: Span,
        name: &str,
        ty: Option<TSType<'a>>,
        value: Expression<'a>,
    ) -> Statement<'a> {
        let declarator = VariableDeclarator::new(
            SPAN,
            BindingPattern::new_binding_identifier(SPAN, self.text(name), self),
            ty.map(|ty| self.annotation(ty)),
            Some(value),
            false,
            self,
        );
        let declaration = Declaration::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            self.vec([declarator]),
            false,
            self,
        );

        self.export(span, declaration)
    }
}
