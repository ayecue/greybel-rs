use std::collections::HashMap;
use std::fmt::{self, Write};

use crate::model::{
    Definition, Field, Payload, ResponseBody, Role, Stream, TypeDeclaration, TypeKind, TypeRef,
};
use crate::versions::AdditionTable;

const FNV_OFFSET: u32 = 0x811c_9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

pub(crate) struct Fnv;

impl Fnv {
    pub(crate) fn of(bytes: &[u8]) -> u32 {
        bytes.iter().fold(FNV_OFFSET, |hash, byte| {
            (hash ^ u32::from(*byte)).wrapping_mul(FNV_PRIME)
        })
    }
}

pub(crate) struct Fingerprint<'d> {
    definition: &'d Definition,
    additions: &'d AdditionTable,
    text: String,
    type_indices: HashMap<&'d str, usize>,
}

impl<'d> Fingerprint<'d> {
    pub(crate) fn of(definition: &'d Definition, additions: &'d AdditionTable) -> u32 {
        let mut fingerprint = Self {
            definition,
            additions,
            text: String::new(),
            type_indices: HashMap::new(),
        };
        let _ = fingerprint.write_definition();

        Fnv::of(fingerprint.text.as_bytes())
    }

    pub(crate) fn of_type(
        definition: &'d Definition,
        additions: &'d AdditionTable,
        name: &'d str,
    ) -> u32 {
        let mut fingerprint = Self {
            definition,
            additions,
            text: String::new(),
            type_indices: HashMap::new(),
        };
        let _ = fingerprint.write_declared_type(name);

        Fnv::of(fingerprint.text.as_bytes())
    }

    fn write_definition(&mut self) -> fmt::Result {
        let definition = self.definition;
        let Some(protocol) = &definition.protocol else {
            let mut types: Vec<&TypeDeclaration> = definition.types.iter().collect();

            types.sort_by(|a, b| a.name.cmp(&b.name));
            for declaration in types {
                write!(self.text, "{}=", declaration.name)?;
                self.write_declared_type(&declaration.name)?;
                writeln!(self.text)?;
            }

            return Ok(());
        };

        writeln!(self.text, "channel {:?}", protocol.channel)?;

        let handshake = &protocol.handshake;

        for (keyword, payload) in [
            ("open", &handshake.open),
            ("ack", &handshake.ack),
            ("refuse", &handshake.refusal),
        ] {
            write!(self.text, "{keyword} ")?;
            match payload {
                Some(ty) => self.write_type(ty)?,
                None => write!(self.text, "-")?,
            }

            writeln!(self.text)?;
        }

        for role in Role::ALL {
            writeln!(self.text, "{}:", role.keyword())?;
            for message in protocol.inbound_messages(role) {
                write!(self.text, "message@{} ", self.additions.message(message))?;
                self.write_fields(&message.fields)?;
                if let Some(field) = &message.stream {
                    write!(self.text, " ~")?;
                    self.write_stream(&field.stream)?;
                }

                if let Some(response) = &message.response {
                    write!(self.text, " -> ")?;
                    match &response.body {
                        ResponseBody::Value(ty) => self.write_type(ty)?,
                        ResponseBody::Stream(stream) => self.write_stream(stream)?,
                    }

                    if let Some(failure) = &response.failure {
                        write!(self.text, " ! ")?;
                        self.write_type(failure)?;
                    }
                }

                writeln!(self.text)?;
            }
        }

        Ok(())
    }

    fn write_stream(&mut self, stream: &'d Stream) -> fmt::Result {
        write!(self.text, "stream(")?;
        for value in [&stream.start, &stream.end] {
            match value {
                Some(ty) => self.write_type(ty)?,
                None => write!(self.text, "_")?,
            }

            write!(self.text, ",")?;
        }

        write!(self.text, ")<")?;
        self.write_type(&stream.item)?;

        write!(self.text, ">")
    }

    fn write_declared_type(&mut self, name: &'d str) -> fmt::Result {
        if let Some(index) = self.type_indices.get(name) {
            return write!(self.text, "#{index}");
        }

        let index = self.type_indices.len();

        self.type_indices.insert(name, index);
        write!(self.text, "#{index}=")?;

        let Some(declaration) = self.definition.declaration(name) else {
            return write!(self.text, "?");
        };

        match &declaration.kind {
            TypeKind::Struct(fields) => {
                write!(self.text, "struct")?;

                self.write_fields(fields)
            }
            TypeKind::Union(variants) => {
                write!(self.text, "union{{")?;
                for variant in variants {
                    write!(self.text, "@{}", self.additions.variant(variant))?;
                    match &variant.payload {
                        Payload::Unit => {}
                        Payload::Value(ty) => {
                            write!(self.text, "(")?;
                            self.write_type(ty)?;
                            write!(self.text, ")")?;
                        }
                        Payload::Fields(fields) => self.write_fields(fields)?,
                    }

                    write!(self.text, ",")?;
                }

                write!(self.text, "}}")
            }
            TypeKind::Enumeration(members) => {
                write!(self.text, "enum{{")?;
                for (index, member) in (0..).zip(members) {
                    let value = member.value.unwrap_or(index);

                    write!(self.text, "@{}={value},", self.additions.member(member))?;
                }

                write!(self.text, "}}")
            }
        }
    }

    fn write_fields(&mut self, fields: &'d [Field]) -> fmt::Result {
        write!(self.text, "{{")?;
        for field in fields {
            write!(self.text, "@{}", self.additions.field(field))?;
            if field.flatten {
                write!(self.text, "flatten ")?;
            }

            if field.optional {
                write!(self.text, "?")?;
            }

            write!(self.text, ":")?;
            self.write_type(&field.ty)?;
            if let Some(default) = self.definition.default_of(field) {
                write!(self.text, "={}", default.canonical())?;
            }

            write!(self.text, ",")?;
        }

        write!(self.text, "}}")
    }

    fn write_type(&mut self, ty: &'d TypeRef) -> fmt::Result {
        match ty {
            TypeRef::Bool => write!(self.text, "bool"),
            TypeRef::Char => write!(self.text, "char"),
            TypeRef::String(_) => write!(self.text, "string"),
            TypeRef::Bytes(_) => write!(self.text, "bytes"),
            TypeRef::Number(kind, _) => write!(self.text, "{}", kind.keyword()),
            TypeRef::List(item, _) => {
                write!(self.text, "list<")?;
                self.write_type(item)?;

                write!(self.text, ">")
            }
            TypeRef::Record(value, _) => {
                write!(self.text, "record<")?;
                self.write_type(value)?;

                write!(self.text, ">")
            }
            TypeRef::Nullable(inner) => {
                self.write_type(inner)?;

                write!(self.text, "?")
            }
            TypeRef::Declared(name) => self.write_declared_type(name),
        }
    }
}
