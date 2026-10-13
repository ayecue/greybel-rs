use std::collections::BTreeSet;

use super::{Encoding, Plan};
use crate::model::{Definition, Payload, TypeKind, TypeRef};

impl<'d> Plan<'d> {
    pub(super) fn requirement_types(definition: &'d Definition) -> BTreeSet<&'d str> {
        let mut requirement_types = BTreeSet::new();

        loop {
            let before = requirement_types.len();

            for declaration in definition.types() {
                let types: Vec<&TypeRef> = match &declaration.kind {
                    TypeKind::Struct(fields) => fields.iter().map(|field| &field.ty).collect(),
                    TypeKind::Union(variants) => variants
                        .iter()
                        .flat_map(|variant| match &variant.payload {
                            Payload::Unit => Vec::new(),
                            Payload::Value(ty) => vec![ty],
                            Payload::Fields(fields) => {
                                fields.iter().map(|field| &field.ty).collect()
                            }
                        })
                        .collect(),
                    TypeKind::Enumeration(_) => Vec::new(),
                };

                if types
                    .iter()
                    .any(|ty| Self::requires_type(ty, &requirement_types))
                {
                    requirement_types.insert(declaration.name.as_str());
                }
            }

            if requirement_types.len() == before {
                return requirement_types;
            }
        }
    }

    fn requires_type(ty: &TypeRef, requirement_types: &BTreeSet<&str>) -> bool {
        match ty {
            TypeRef::Declared(name) => requirement_types.contains(name.as_str()),
            TypeRef::List(item, _) | TypeRef::Record(item, _) | TypeRef::Nullable(item)
                if Self::requires_type(item, requirement_types) =>
            {
                true
            }
            ty => ty.has_requirements(),
        }
    }

    #[must_use]
    pub fn requires(&self, encoding: &Encoding<'_>) -> bool {
        match encoding {
            Encoding::Struct(name) | Encoding::Union(name) => self.requirement_types.contains(name),
            Encoding::List(item, _) | Encoding::Record(item, _) | Encoding::Nullable(item)
                if self.requires(item) =>
            {
                true
            }
            encoding => encoding.has_requirements(),
        }
    }

    /// Returns unique declared-type pattern sources in declaration traversal order.
    #[must_use]
    pub fn patterns(&self) -> Vec<&'d str> {
        let mut patterns = Vec::new();
        let mut add = |ty: &'d TypeRef| Self::patterns_of(ty, &mut patterns);

        for declaration in self.definition.types() {
            match &declaration.kind {
                TypeKind::Struct(fields) => fields.iter().for_each(|field| add(&field.ty)),
                TypeKind::Union(variants) => {
                    for variant in variants {
                        match &variant.payload {
                            Payload::Unit => {}
                            Payload::Value(ty) => add(ty),
                            Payload::Fields(fields) => {
                                for field in fields {
                                    add(&field.ty);
                                }
                            }
                        }
                    }
                }
                TypeKind::Enumeration(_) => {}
            }
        }

        if let Some(protocol) = self.definition.protocol() {
            for message in protocol.messages() {
                message.types().for_each(&mut add);
            }
        }

        patterns
    }

    fn patterns_of(ty: &'d TypeRef, patterns: &mut Vec<&'d str>) {
        match ty {
            TypeRef::String(requirements) => {
                if let Some(pattern) = &requirements.pattern
                    && !patterns.contains(&pattern.as_str())
                {
                    patterns.push(pattern);
                }
            }
            TypeRef::List(item, _) | TypeRef::Record(item, _) | TypeRef::Nullable(item) => {
                Self::patterns_of(item, patterns);
            }
            _ => {}
        }
    }
}
