use greybel_core::syntax::greybel::{TypeBody, TypeMember};
use greybel_core::{Ast, NodeKind, Span};

use super::Inference;
use crate::annotation::{
    AliasDoc, FunctionDoc, ParamDoc, PropertyDoc, TextPosition, TypeDoc, TypeExpr,
};
use crate::types::TypeId;

#[derive(Debug, Clone)]
pub(super) struct TypeAlias {
    parameters: Vec<String>,
    ty: TypeExpr,
}

impl Inference<'_> {
    pub(super) fn type_statement_docs(&mut self, document: usize) -> Vec<(Span, TypeDoc)> {
        let ast = self.documents[document].ast;
        let mut docs = Vec::new();
        let comment_aliases: Vec<AliasDoc> = self.documents[document]
            .comments
            .blocks
            .iter()
            .flat_map(|block| block.doc.aliases.iter().cloned())
            .collect();

        for alias in comment_aliases {
            self.type_aliases.insert(
                alias.name,
                TypeAlias {
                    parameters: alias.type_parameters,
                    ty: alias.ty,
                },
            );
        }

        for &node in ast.type_syntax.declarations() {
            let NodeKind::TypeStatement(statement) = ast.kind(node) else {
                continue;
            };
            let name = ast.slice(statement.name).to_owned();
            let parameters = statement
                .type_parameters
                .iter()
                .map(|&parameter| ast.slice(parameter).to_owned())
                .collect();

            match &statement.body {
                TypeBody::Alias(ty) => {
                    if let Some(ty) = TypeExpr::parse(ast.slice(*ty)) {
                        self.type_aliases.insert(name, TypeAlias { parameters, ty });
                    }
                }
                TypeBody::Members { extends, members } => {
                    docs.push((
                        ast.span(node),
                        Self::type_doc(ast, name, parameters, *extends, members),
                    ));
                }
            }
        }

        docs
    }

    fn type_doc(
        ast: &Ast,
        name: String,
        type_parameters: Vec<String>,
        extends: Option<Span>,
        members: &[TypeMember],
    ) -> TypeDoc {
        let parse = |span: Option<Span>| span.and_then(|span| TypeExpr::parse(ast.slice(span)));
        let mut doc = TypeDoc {
            name,
            line: 0,
            virtual_type: true,
            type_parameters,
            extends: parse(extends),
            properties: Vec::new(),
            functions: Vec::new(),
            description: None,
        };

        for member in members {
            let member_name = ast.slice(member.name).to_owned();

            match &member.parameters {
                Some(parameters) => doc.functions.push(FunctionDoc {
                    line: 0,
                    name: member_name,
                    params: parameters
                        .iter()
                        .map(|&(name, ty)| ParamDoc {
                            name: ast.slice(name).to_owned(),
                            name_position: TextPosition::default(),
                            ty: parse(ty),
                            description: None,
                            optional: false,
                        })
                        .collect(),
                    returns: parse(member.ty),
                    mutated_parameter: None,
                    description: None,
                    examples: Vec::new(),
                }),
                None => {
                    if let Some(ty) = parse(member.ty) {
                        doc.properties.push(PropertyDoc {
                            line: 0,
                            path: vec![member_name],
                            ty,
                            description: None,
                            optional: member.optional,
                            readonly: member.readonly,
                        });
                    }
                }
            }
        }

        doc
    }

    pub(super) fn alias_type(&mut self, name: &str, arguments: &[TypeId]) -> Option<TypeId> {
        let alias = self.type_aliases.get(name)?.clone();

        if self.resolving_aliases.iter().any(|alias| alias == name) {
            return Some(TypeId::UNKNOWN);
        }

        let depth = self.type_parameters_in_scope.len();

        for (index, parameter) in alias.parameters.iter().enumerate() {
            let argument = arguments.get(index).copied().unwrap_or(TypeId::ANY);

            self.type_parameters_in_scope
                .push((parameter.clone(), argument));
        }

        self.resolving_aliases.push(name.to_owned());

        let ty = self.resolve_type(&alias.ty);

        self.resolving_aliases.pop();
        self.type_parameters_in_scope.truncate(depth);

        Some(ty)
    }
}
