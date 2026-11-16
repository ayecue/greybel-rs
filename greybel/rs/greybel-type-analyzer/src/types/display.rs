use std::fmt;

use greybel_symbols::annotations::{ANY, NEVER, UNKNOWN};
use greybel_symbols::types;

use super::{FunctionRef, LiteralType, Type, TypeId, TypeTable};
use crate::model::{
    FunctionSignature, Key, MapId, MapOrigin, SemanticModel, SignatureParameter, SignatureType,
};

const MAX_ENTRIES: usize = 16;

#[derive(Clone, Copy)]
pub(crate) struct TypeNotation<'a> {
    model: &'a SemanticModel,
    types: &'a TypeTable,
}

impl<'a> TypeNotation<'a> {
    pub(crate) fn new(model: &'a SemanticModel, types: &'a TypeTable) -> Self {
        Self { model, types }
    }

    pub(crate) fn display(self, ty: TypeId) -> TypeDisplay<'a> {
        TypeDisplay { notation: self, ty }
    }

    pub(crate) fn format_signature(self, signature: &FunctionSignature<'_>) -> String {
        let parameters: Vec<String> = signature
            .parameters
            .iter()
            .map(|parameter| self.format_parameter(parameter))
            .collect();

        format!(
            "({}): {}",
            parameters.join(", "),
            self.format_signature_type(signature.returns)
        )
    }

    pub(crate) fn format_parameter(self, parameter: &SignatureParameter<'_>) -> String {
        let mut label = format!(
            "{}: {}",
            parameter.name,
            self.format_signature_type(parameter.ty)
        );

        match parameter.default {
            Some(default) => {
                label.push_str(" = ");
                label.push_str(&default.to_string());
            }
            None if parameter.optional => label.push('?'),
            None => {}
        }

        label
    }

    fn format_signature_type(self, ty: SignatureType<'_>) -> String {
        match ty {
            SignatureType::Value(ty) => self.display(ty).to_string(),
            SignatureType::Guard(guard) => guard.to_string(),
            SignatureType::Api(types) => {
                let types: Vec<String> = types.iter().map(ToString::to_string).collect();

                types.join(" or ")
            }
        }
    }

    pub(crate) fn map_content(self, ty: TypeId) -> Option<String> {
        let mut entries: Vec<(String, Vec<TypeId>)> = Vec::new();

        for variant in self.types.variants(ty) {
            let content = match *self.types.get(variant) {
                Type::Object(_) => variant,
                Type::Function(function) => match self.return_type(function) {
                    Some(returns) => returns,
                    None => continue,
                },
                _ => continue,
            };

            for map in self.types.object_maps(content) {
                self.collect_entries(map, &mut entries);
            }
        }

        self.format_entries(&entries)
    }

    /// The entries of a custom type's map and of the types it extends, as hovers show them.
    pub(crate) fn custom_type_content(self, map: MapId) -> Option<String> {
        let mut entries: Vec<(String, Vec<TypeId>)> = Vec::new();

        for map in self.model.isa_chain(self.types, map) {
            self.push_entries(map, &mut entries);
        }

        self.format_entries(&entries)
    }

    fn format_entries(self, entries: &[(String, Vec<TypeId>)]) -> Option<String> {
        if entries.is_empty() {
            return None;
        }

        let mut lines: Vec<String> = entries
            .iter()
            .take(MAX_ENTRIES)
            .map(|(key, types)| {
                let mut names: Vec<String> = types
                    .iter()
                    .map(|&ty| self.display(ty).to_string())
                    .collect();

                names.sort();
                names.dedup();

                format!("  {key}: {}", names.join(" or "))
            })
            .collect();

        if entries.len() > MAX_ENTRIES {
            lines.push(format!("  ... {} more", entries.len() - MAX_ENTRIES));
        }

        Some(format!("{{\n{}\n}}", lines.join(",\n")))
    }

    fn collect_entries(self, map: MapId, entries: &mut Vec<(String, Vec<TypeId>)>) {
        for map in self.model.isa_chain(self.types, map) {
            let data = self.model.map(map);

            if matches!(
                data.origin,
                MapOrigin::Custom(_)
                    | MapOrigin::GenericInstance(_)
                    | MapOrigin::NestedKey { .. }
                    | MapOrigin::Prototype(_)
            ) {
                continue;
            }

            self.push_entries(map, entries);
        }
    }

    fn push_entries(self, map: MapId, entries: &mut Vec<(String, Vec<TypeId>)>) {
        let data = self.model.map(map);
        let dynamic_value = self.model.slot_type(data.dynamic_value);
        let dynamic = (dynamic_value != TypeId::UNKNOWN).then(|| {
            let key = self.display(self.model.slot_type(data.dynamic_key));

            (format!("[{key}]"), dynamic_value)
        });
        let own_entries = data.entries.iter().map(|(key, &slot)| {
            let key = match key {
                Key::String(name) => format!("\"{}\"", name.replace('"', "\"\"")),
                Key::Number(bits) => f64::from_bits(*bits).to_string(),
            };

            (key, self.model.slot_type(slot))
        });

        for (key, ty) in own_entries.chain(dynamic) {
            match entries.iter_mut().find(|(entry_key, _)| *entry_key == key) {
                Some((_, types)) => types.push(ty),
                None => entries.push((key, vec![ty])),
            }
        }
    }

    fn return_type(self, function: FunctionRef) -> Option<TypeId> {
        match function {
            FunctionRef::Body(id) => {
                Some(self.model.slot_type(self.model.function(id).return_slot))
            }
            FunctionRef::Signature(id) => Some(self.model.declared_function(id).return_type),
            FunctionRef::ApiMethod(id) => Some(self.model.api_method(id).returns),
            FunctionRef::Api(function) => self.model.singleton_type(function),
            FunctionRef::Any => None,
        }
    }

    fn write_object(self, f: &mut fmt::Formatter<'_>, map: MapId) -> fmt::Result {
        match self.model.map(map).origin {
            MapOrigin::Custom(name) => f.write_str(self.model.name(name)),
            MapOrigin::GenericInstance(id) => {
                let specialization = self.model.generic_instance(id);

                self.write_object(f, specialization.generic)?;

                let arguments: Vec<String> = specialization
                    .arguments
                    .iter()
                    .map(|&argument| self.display(argument).to_string())
                    .collect();

                write!(f, "<{}>", arguments.join(","))
            }
            _ => f.write_str(types::MAP),
        }
    }
}

pub struct TypeDisplay<'a> {
    notation: TypeNotation<'a>,
    ty: TypeId,
}

impl fmt::Display for TypeDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let notation = self.notation;

        match notation.types.get(self.ty) {
            Type::Unknown => f.write_str(UNKNOWN),
            Type::Any => f.write_str(ANY),
            Type::Never => f.write_str(NEVER),
            Type::Null => f.write_str(types::NULL),
            Type::Number => f.write_str(types::NUMBER),
            Type::String => f.write_str(types::STRING),
            Type::Literal(LiteralType::String(text)) => {
                write!(f, "\"{}\"", text.replace('"', "\"\""))
            }
            Type::Literal(LiteralType::Number(bits)) => write!(f, "{}", f64::from_bits(*bits)),
            Type::List(item) => write!(f, "{}<{}>", types::LIST, notation.display(*item)),
            Type::Map { key, value } => {
                write!(
                    f,
                    "{}<{},{}>",
                    types::MAP,
                    notation.display(*key),
                    notation.display(*value)
                )
            }
            Type::Object(map) => notation.write_object(f, *map),
            Type::Class(name) => f.write_str(name),
            Type::TypeParameter { name, .. } => f.write_str(name),
            Type::Function(function) => {
                f.write_str(types::FUNCTION)?;

                match notation.model.signature(*function, false) {
                    Some(signature) => f.write_str(&notation.format_signature(&signature)),
                    None => Ok(()),
                }
            }
            Type::Union(variants) => {
                let mut names: Vec<String> = variants
                    .iter()
                    .map(|&variant| notation.display(variant).to_string())
                    .collect();

                names.sort();
                names.dedup();

                f.write_str(&names.join(" or "))
            }
        }
    }
}
