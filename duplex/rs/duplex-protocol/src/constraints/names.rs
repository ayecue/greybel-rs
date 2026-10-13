use std::collections::HashSet;

use super::ConstraintTable;
use crate::error::DefinitionError;
use crate::model::Span;
use crate::names::Symbol;
use crate::syntax::{RESULT, STREAM};

impl ConstraintTable<'_> {
    pub(super) fn verify_names(&self) -> Result<(), DefinitionError> {
        let mut names = HashSet::new();
        let mut declare = |name: String, span: Span| {
            if !Self::is_pascal_case(&name) {
                return Err(DefinitionError::new(
                    span,
                    format!("`{name}` is a name in PascalCase"),
                ));
            }

            if [RESULT, STREAM].contains(&name.as_str()) {
                return Err(DefinitionError::new(
                    span,
                    format!("`{name}` is a keyword of responses"),
                ));
            }

            if !names.insert(name.clone()) {
                return Err(DefinitionError::new(
                    span,
                    format!("`{name}` is declared twice"),
                ));
            }

            Ok(())
        };

        for declaration in &self.definition.types {
            declare(declaration.name.clone(), declaration.span)?;
        }

        if let Some(protocol) = &self.definition.protocol {
            declare(protocol.name.clone(), protocol.span)?;
            declare(Symbol::Peers(protocol).pascal(), protocol.span)?;
            declare(Symbol::Handshake(protocol).pascal(), protocol.span)?;
            for peer in &protocol.peers {
                declare(peer.name.clone(), peer.span)?;
                declare(Symbol::Router(peer).pascal(), peer.span)?;
                declare(Symbol::Channel(peer).pascal(), peer.span)?;
                declare(Symbol::Notification(peer).pascal(), peer.span)?;
            }

            for message in &protocol.messages {
                declare(message.name.clone(), message.span)?;
            }
        }

        Ok(())
    }

    pub(super) fn verify_member_name<'n>(
        names: &mut HashSet<&'n str>,
        name: &'n str,
        span: Span,
    ) -> Result<(), DefinitionError> {
        if !Self::is_pascal_case(name) {
            return Err(DefinitionError::new(
                span,
                format!("`{name}` is a name in PascalCase"),
            ));
        }

        if !names.insert(name) {
            return Err(DefinitionError::new(
                span,
                format!("`{name}` is declared twice"),
            ));
        }

        Ok(())
    }

    pub(super) fn is_capitalized_snake_case(name: &str) -> bool {
        name.starts_with(|char: char| char.is_ascii_uppercase())
            && name
                .chars()
                .all(|char| char.is_ascii_uppercase() || char.is_ascii_digit() || char == '_')
    }

    pub(super) fn is_pascal_case(name: &str) -> bool {
        name.starts_with(|char: char| char.is_ascii_uppercase())
            && name.chars().all(|char| char.is_ascii_alphanumeric())
    }

    pub(super) fn is_snake_case(name: &str) -> bool {
        name.starts_with(|char: char| char.is_ascii_lowercase())
            && name
                .chars()
                .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '_')
    }
}
