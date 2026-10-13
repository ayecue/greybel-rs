mod literals;

use greybel_core::lexer::{KEYWORDS, Keyword};
use greybel_core::syntax::Literal;
use greybel_core::{Ast, Atom, NodeId, NodeKind};
use greybel_symbols::names::{LOCALS, OUTER, SELF, SUPER};
use greybel_symbols::types::{FUNC_REF, LIST, MAP, NUMBER, STRING};
use literals::LiteralTable;

use crate::bundle::{MULTILINE_STRING, RUNTIME_NAMES};
use crate::emit::LineBreakStyle;
use crate::names::{NameTable, variable_charset};
use crate::options::TranspileOptions;

const RESERVED_NAMES: &[&str] = &[
    LOCALS, OUTER, SELF, SUPER, STRING, LIST, MAP, NUMBER, FUNC_REF,
];

#[derive(Debug, Clone)]
pub struct Minification {
    variables: NameTable,
    literals: LiteralTable,
}

impl Minification {
    #[must_use]
    pub fn analyze(asts: &[&Ast], options: &TranspileOptions) -> Self {
        let forbidden = RESERVED_NAMES
            .iter()
            .copied()
            .chain(KEYWORDS.iter().map(|keyword| keyword.as_str()))
            .chain([Keyword::End.as_str()])
            .chain(options.excluded_names.iter().map(String::as_str));
        let names: Vec<&str> = asts.iter().flat_map(|ast| variable_names(ast)).collect();
        let mut variables = NameTable::with_capacity(
            variable_charset(options.obfuscation),
            RUNTIME_NAMES.len() + names.len(),
        )
        .forbid(forbidden);

        for name in RUNTIME_NAMES.iter().copied().chain(names) {
            variables.insert(name);
        }

        let literal_count = asts.iter().map(|ast| ast.literals.len()).sum();
        let mut literals = LiteralTable::with_capacity(literal_count);

        for ast in asts {
            for &id in &ast.literals {
                if let NodeKind::Literal(literal) = ast.kind(id) {
                    literals.add(literal, ast.text(id).unwrap_or_default(), &mut variables);
                }
            }
        }

        variables.insert(MULTILINE_STRING);

        Self {
            variables,
            literals,
        }
    }

    #[must_use]
    pub fn rename<'a>(&'a self, name: &'a str) -> &'a str {
        self.variables.get(name).unwrap_or(name)
    }

    #[must_use]
    pub fn hoisted_variable(&self, literal: &Literal, source: &str) -> Option<&str> {
        self.literals.hoisted_variable(literal, source)
    }

    pub fn hoisted_literals<'a>(
        &'a self,
        line_breaks: LineBreakStyle<'a>,
    ) -> impl Iterator<Item = (&'a str, String)> {
        self.literals.hoisted_entries(line_breaks)
    }
}

fn variable_names(ast: &Ast) -> impl Iterator<Item = &str> {
    let scopes = std::iter::once(ast.scope()).chain(ast.scopes.iter().filter_map(|&scope| {
        ast.kind(scope)
            .as_node()
            .as_scope()
            .map(|scope| scope.scope())
    }));
    let mut atom_flags = vec![false; ast.nodes().atoms().len()];

    scopes
        .flat_map(|scope| ast.list(scope.namespaces).iter().copied())
        .filter_map(|id| identifier_atom(ast, id))
        .filter(move |atom| !std::mem::replace(&mut atom_flags[atom.index()], true))
        .map(|atom| ast.resolve(atom))
}

fn identifier_atom(ast: &Ast, id: NodeId) -> Option<Atom> {
    match ast.kind(id) {
        NodeKind::Identifier(identifier) => Some(identifier.name),
        NodeKind::Member(member) => identifier_atom(ast, member.identifier),
        _ => None,
    }
}
