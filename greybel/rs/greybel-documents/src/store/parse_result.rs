use std::sync::Arc;

use greybel_core::{Ast, ParseError, Parser};

use crate::options::DocumentOptions;

#[derive(Debug, Clone)]
pub struct ParseResult {
    ast: Option<Arc<Ast>>,
    errors: Vec<ParseError>,
}

impl ParseResult {
    #[must_use]
    pub fn new(filename: &str, text: &str, options: &DocumentOptions) -> Self {
        let parser = Parser::new(text, options.parser_options(filename));

        if options.unsafe_mode {
            let output = parser.parse_unsafe();
            let errors = output.all_errors().cloned().collect();

            return Self {
                ast: Some(Arc::new(output.ast)),
                errors,
            };
        }

        match parser.parse() {
            Ok(ast) => Self {
                ast: Some(Arc::new(ast)),
                errors: Vec::new(),
            },
            Err(error) => Self {
                ast: None,
                errors: vec![error],
            },
        }
    }

    #[must_use]
    pub fn ast(&self) -> Option<&Arc<Ast>> {
        self.ast.as_ref()
    }

    #[must_use]
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}
