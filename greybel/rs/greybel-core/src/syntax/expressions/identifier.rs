use crate::ast::{AstNode, Atom, DumpContext, NodeFieldContainer, NodeId};
use crate::diagnostics::ExpectationDiagnostic;
use crate::error::ParseResult;
use crate::parser::Parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentifierKind {
    Variable,
    Argument,
    ForInVariable,
    Property,
}

impl IdentifierKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            IdentifierKind::Variable => "variable",
            IdentifierKind::Argument => "argument",
            IdentifierKind::ForInVariable => "for-in-variable",
            IdentifierKind::Property => "property",
        }
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Identifier {
    pub name: Atom,
    pub kind: IdentifierKind,
}

impl AstNode for Identifier {
    fn type_name(&self) -> &'static str {
        "Identifier"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{}]",
            ctx.location(),
            ctx.resolve(self.name)
        )
    }
}

impl Identifier {
    pub fn parse(parser: &mut Parser<'_>, kind: IdentifierKind) -> ParseResult<NodeId> {
        let Some((name, hash)) = parser
            .token()
            .as_identifier()
            .map(|token| (token.name(parser.source()), token.hash()))
        else {
            let got = parser.token().describe(parser.source());
            let diagnostic = match kind {
                IdentifierKind::Argument => ExpectationDiagnostic::Parameter { got },
                _ => ExpectationDiagnostic::Identifier { got },
            };
            let span = parser.token().span();

            parser.raise_at(diagnostic, span)?;

            return parser.parse_invalid_code();
        };
        let name = parser.stream.nodes.insert_atom_with_hash(name, hash);

        parser.advance()?;

        let identifier = parser.alloc(Identifier { name, kind }, parser.prev().span());

        if kind != IdentifierKind::Property {
            parser.scope_data_mut().namespaces.push(identifier);
        }

        Ok(identifier)
    }
}
