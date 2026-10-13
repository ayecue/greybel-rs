mod declarations;
mod fields;
mod finite;
mod names;
mod requirements;
mod tags;
mod types;
mod values;

use std::collections::{HashMap, HashSet};

use crate::error::DefinitionError;
use crate::model::{Definition, TypeDeclaration};

pub(crate) struct ConstraintTable<'a> {
    definition: &'a Definition,
    types: HashMap<&'a str, &'a TypeDeclaration>,
}

impl<'a> ConstraintTable<'a> {
    pub(crate) fn of(definition: &'a Definition) -> Self {
        Self {
            definition,
            types: definition
                .types
                .iter()
                .map(|declaration| (declaration.name.as_str(), declaration))
                .collect(),
        }
    }

    pub(crate) fn verify(&self) -> Result<(), DefinitionError> {
        self.verify_names()?;
        self.verify_constants()?;
        self.verify_tags()?;
        for declaration in &self.definition.types {
            self.verify_declaration(declaration)?;
        }

        if let Some(protocol) = &self.definition.protocol {
            if self.definition.core.is_none() {
                return Err(DefinitionError::new(
                    protocol.span,
                    "a protocol names the core it builds on, as `use core 1.0;`",
                ));
            }

            let mut transports = HashSet::new();

            for flavour in &protocol.flavours {
                if !Self::is_snake_case(&flavour.transport) {
                    return Err(DefinitionError::new(
                        flavour.span,
                        format!(
                            "`{}` is a transport's name in snake_case",
                            flavour.transport
                        ),
                    ));
                }

                if !transports.insert(flavour.transport.as_str()) {
                    return Err(DefinitionError::new(
                        flavour.span,
                        format!("the flavour of `{}` is declared twice", flavour.transport),
                    ));
                }
            }

            let handshake = &protocol.handshake;

            for payload in [&handshake.open, &handshake.ack].into_iter().flatten() {
                self.verify_type(payload, protocol.span)?;
            }

            if let Some(refusal) = &handshake.refusal {
                self.verify_declared_failure(refusal, protocol.span, "a refusal is")?;
            }

            for message in &protocol.messages {
                self.verify_fields(&message.fields)?;
                self.verify_limit(protocol, message)?;

                let failure = message.failure();

                for ty in message.types().skip(message.fields.len()) {
                    if Some(ty) != failure {
                        self.verify_type(ty, message.span)?;
                    }
                }

                if let Some(failure) = failure {
                    self.verify_failure(failure, message.span)?;
                }
            }
        }

        for declaration in &self.definition.types {
            self.verify_finite(declaration, &mut Vec::new())?;
        }

        Ok(())
    }
}
