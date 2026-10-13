//! C# protocol value encoding and decoding statement generator.

mod length;
mod patterns;
mod property;
mod range;

use std::cell::Cell;

use duplex_protocol::{
    Bit, BlockPlan, Encoding, FieldPlan, FieldValue, FieldsPlan, PatternTable, Plan,
    StringRequirementContainer,
};
use length::{length_arguments, read_length, write_length};
use patterns::{base64, table_bytes};
pub(crate) use property::{Property, Shape};
use range::{bounds_of, range_check};

use crate::code::Code;
use crate::types;

const WRITER: &str = "writer";
const READER: &str = "reader";
const PATTERNS: &str = "Patterns";

pub(crate) struct Codec<'p, 'd> {
    plan: &'p Plan<'d>,
    locals: Cell<usize>,
}

impl<'p, 'd> Codec<'p, 'd> {
    pub(crate) fn new(plan: &'p Plan<'d>) -> Self {
        Self {
            plan,
            locals: Cell::new(0),
        }
    }

    pub(crate) fn reset_locals(&self) {
        self.locals.set(0);
    }

    fn local(&self, base: &str) -> String {
        let index = self.locals.get();

        self.locals.set(index + 1);

        format!("{base}{index}")
    }

    /// Writes compiled pattern fields for patterns required by module types.
    pub(crate) fn patterns(&self, code: &mut Code) {
        let patterns = self.plan.patterns();

        if patterns.is_empty() {
            return;
        }

        code.block(format!("file static class {PATTERNS}"), |code| {
            for (index, source) in patterns.iter().enumerate() {
                let table =
                    PatternTable::compile(source).expect("the constraints compile patterns");

                code.line(format!(
                    "public static readonly global::Duplex.Pattern P{index} = global::Duplex.Pattern.Decode(\"{}\");",
                    base64(&table_bytes(&table))
                ));
            }
        });
    }

    fn pattern(&self, source: &str) -> String {
        let index = self
            .plan
            .patterns()
            .iter()
            .position(|pattern| *pattern == source)
            .expect("the plan lists every pattern");

        format!("{PATTERNS}.P{index}")
    }

    fn within(
        &self,
        code: &mut Code,
        side: &str,
        position: usize,
        encoding: &Encoding<'_>,
        body: impl FnOnce(&mut Code),
    ) {
        if !self.plan.requires(encoding) {
            body(code);

            return;
        }

        code.line(format!("{side}.Enter({position});"));
        body(code);
        code.line(format!("{side}.Exit();"));
    }

    fn each_item(
        &self,
        code: &mut Code,
        side: &str,
        item: &Encoding<'_>,
        head: String,
        body: impl FnOnce(&mut Code),
    ) {
        let requires = self.plan.requires(item);

        if requires {
            code.line(format!("{side}.Enter(-1);"));
        }

        code.block(head, |code| {
            if requires {
                code.line(format!("{side}.Advance();"));
            }

            body(code);
        });
        if requires {
            code.line(format!("{side}.Exit();"));
        }
    }

    /// Writes statements encoding `source` properties for fields in `plan`.
    pub(crate) fn write_fields(
        &self,
        code: &mut Code,
        plan: &FieldsPlan<'_>,
        owner: &str,
        source: &str,
    ) {
        for block in &plan.blocks {
            if block.since == 0 {
                self.write_block(code, block, owner, source);
            } else {
                code.block(format!("if ({WRITER}.Minor >= {})", block.since), |code| {
                    self.write_block(code, block, owner, source);
                });
            }
        }
    }

    fn write_block(&self, code: &mut Code, block: &BlockPlan<'_>, owner: &str, source: &str) {
        for byte in 0..block.flags.len() {
            let terms: Vec<String> = block
                .fields
                .iter()
                .flat_map(|field| Self::flag_terms(&Property::of(field, owner), source, byte))
                .collect();

            if terms.is_empty() {
                code.line(format!("{WRITER}.WriteByte(0);"));
            } else {
                code.line(format!(
                    "{WRITER}.WriteByte((byte)({}));",
                    terms.join(" | ")
                ));
            }
        }

        for field in &block.fields {
            self.write_field(code, field, owner, source);
        }
    }

    fn flag_terms(property: &Property<'_, '_>, source: &str, byte: usize) -> Vec<String> {
        let access = format!("{source}.{}", property.name);
        let present = || match property.shape() {
            Shape::Both => format!("{access}.HasValue"),
            _ => format!("{access} != null"),
        };
        let non_null = || match property.shape() {
            Shape::Both => format!("({access}.HasValue && {access}.Value != null)"),
            _ => format!("{access} != null"),
        };
        let flag = || match property.shape() {
            Shape::Always => access.clone(),
            Shape::Both => format!("({access}.HasValue && {access}.Value == true)"),
            Shape::Present | Shape::NonNull => format!("{access} == true"),
        };
        let presence = property.plan.presence;
        let value_flag = match property.plan.value {
            FieldValue::Flag(bit) => Some(bit),
            _ => None,
        };

        [
            presence.present.map(|bit| (bit, present())),
            presence.non_null.map(|bit| (bit, non_null())),
            value_flag.map(|bit| (bit, flag())),
        ]
        .into_iter()
        .flatten()
        .filter(|(bit, _)| bit.byte == byte)
        .map(|(bit, condition)| format!("({condition} ? {} : 0)", bit.mask))
        .collect()
    }

    fn write_field(&self, code: &mut Code, field: &FieldPlan<'_>, owner: &str, source: &str) {
        let ty = match &field.value {
            FieldValue::Flag(_) => return,
            FieldValue::Flatten { fields, .. } => {
                self.within(code, WRITER, field.position, &field.ty, |code| {
                    self.write_fields(code, fields, owner, source);
                });

                return;
            }
            FieldValue::Value(ty) => ty,
        };
        let property = Property::of(field, owner);
        let access = format!("{source}.{}", property.name);
        let value_type = types::is_value_type(ty);
        let (condition, value) = match property.shape() {
            Shape::Always => (None, access.clone()),
            Shape::Present | Shape::NonNull => (
                Some(format!("{access} != null")),
                if value_type {
                    format!("{access}.Value")
                } else {
                    format!("{access}!")
                },
            ),
            Shape::Both => (
                Some(format!("{access}.HasValue && {access}.Value != null")),
                if value_type {
                    format!("{access}.Value.Value")
                } else {
                    format!("{access}.Value!")
                },
            ),
        };

        self.within(
            code,
            WRITER,
            field.position,
            &field.ty,
            |code| match condition {
                None => self.write_value(code, ty, &value),
                Some(condition) => code.block(format!("if ({condition})"), |code| {
                    self.write_value(code, ty, &value);
                }),
            },
        );
    }

    /// Writes statements encoding C# expression `value` with `encoding`.
    pub(crate) fn write_value(&self, code: &mut Code, encoding: &Encoding<'_>, value: &str) {
        match encoding {
            Encoding::Bool => code.line(format!("{WRITER}.WriteBool({value});")),
            Encoding::Char => code.line(format!("{WRITER}.WriteChar({value});")),
            Encoding::String(requirements) if !requirements.is_any() => code.line(format!(
                "{WRITER}.WriteValidatedString({value}, {});",
                self.string_arguments(requirements)
            )),
            Encoding::String(_) => code.line(format!("{WRITER}.WriteString({value});")),
            Encoding::Bytes(length) if !length.is_any() => code.line(format!(
                "{WRITER}.WriteValidatedBytes({value}, {});",
                length_arguments(*length)
            )),
            Encoding::Bytes(_) => code.line(format!("{WRITER}.WriteBytes({value});")),
            Encoding::Number(kind, _) => {
                let value = match bounds_of(*kind, encoding) {
                    Some(bounds) => range_check(WRITER, *kind, value, &bounds),
                    None => value.to_owned(),
                };

                code.line(format!(
                    "{WRITER}.Write{}({value});",
                    types::number_name(*kind)
                ));
            }
            Encoding::Enumeration(name) => {
                code.line(format!("{}.Write({WRITER}, {value});", codec_class(name)));
            }
            Encoding::Struct(_) | Encoding::Union(_) => {
                code.line(format!("{value}.Write({WRITER});"));
            }
            Encoding::Nullable(inner) => {
                let local = self.local("value");

                code.line(format!("var {local} = {value};"));
                code.block(format!("if ({local} == null)"), |code| {
                    code.line(format!("{WRITER}.WriteByte(0);"));
                });
                code.block("else", |code| {
                    code.line(format!("{WRITER}.WriteByte(1);"));

                    let present = if types::is_value_type(inner) {
                        format!("{local}.Value")
                    } else {
                        local.clone()
                    };

                    self.write_value(code, inner, &present);
                });
            }
            Encoding::List(item, length) => {
                let items = self.local("items");
                let local = self.local("item");

                code.line(format!("var {items} = {value};"));
                code.line(write_length(&format!("{items}.Count"), *length));
                self.each_item(
                    code,
                    WRITER,
                    item,
                    format!("foreach (var {local} in {items})"),
                    |code| {
                        self.write_value(code, item, &local);
                    },
                );
            }
            Encoding::Record(item, length) => {
                let record = self.local("record");
                let entry = self.local("entry");

                code.line(format!("var {record} = {value};"));
                code.line(write_length(&format!("{record}.Count"), *length));
                self.each_item(
                    code,
                    WRITER,
                    item,
                    format!("foreach (var {entry} in {record})"),
                    |code| {
                        code.line(format!("{WRITER}.WriteString({entry}.Key);"));
                        self.write_value(code, item, &format!("{entry}.Value"));
                    },
                );
            }
        }
    }

    fn string_arguments(&self, requirements: &StringRequirementContainer) -> String {
        let pattern = requirements
            .pattern
            .as_deref()
            .map_or_else(|| "null".to_owned(), |pattern| self.pattern(pattern));

        format!("{}, {pattern}", length_arguments(requirements.length))
    }

    /// Decodes `plan` fields into locals and returns local names in class property order.
    pub(crate) fn read_fields(
        &self,
        code: &mut Code,
        plan: &FieldsPlan<'_>,
        owner: &str,
    ) -> Vec<String> {
        let mut locals = Vec::new();

        for block in &plan.blocks {
            if block.since == 0 {
                locals.extend(self.read_block(code, block, owner, true));
                continue;
            }

            for field in Self::block_properties(block) {
                let property = Property::of(field, owner);

                code.line(format!(
                    "{} {} = {};",
                    property.ty(),
                    Self::field_local(&property),
                    property.absent()
                ));
            }

            let mut block_locals = Vec::new();

            code.block(format!("if ({READER}.Minor >= {})", block.since), |code| {
                block_locals = self.read_block(code, block, owner, false);
            });
            locals.extend(block_locals);
        }

        locals
    }

    fn block_properties<'b, 'e>(block: &'b BlockPlan<'e>) -> Vec<&'b FieldPlan<'e>> {
        let mut properties = Vec::new();

        for field in &block.fields {
            match &field.value {
                FieldValue::Flatten { fields, .. } => properties.extend(fields.properties()),
                _ => properties.push(field),
            }
        }

        properties
    }

    fn field_local(property: &Property<'_, '_>) -> String {
        format!("field{}", property.name)
    }

    fn read_block(
        &self,
        code: &mut Code,
        block: &BlockPlan<'_>,
        owner: &str,
        declares: bool,
    ) -> Vec<String> {
        let flags: Vec<String> = block
            .flags
            .iter()
            .map(|mask| {
                let local = self.local("flags");

                code.line(format!("var {local} = {READER}.ReadFlags({mask});"));

                local
            })
            .collect();
        let mut locals = Vec::new();

        for field in &block.fields {
            if let FieldValue::Flatten { fields, .. } = &field.value {
                self.within(code, READER, field.position, &field.ty, |code| {
                    locals.extend(self.read_fields(code, fields, owner));
                });
                continue;
            }

            let property = Property::of(field, owner);
            let local = Self::field_local(&property);

            self.within(code, READER, field.position, &field.ty, |code| {
                self.read_field(code, &property, &flags, &local, declares);
            });
            locals.push(local);
        }

        locals
    }

    fn read_field(
        &self,
        code: &mut Code,
        property: &Property<'_, '_>,
        flags: &[String],
        local: &str,
        declares: bool,
    ) {
        let assign = |value: String| {
            if declares {
                format!("{} {local} = {value};", property.ty())
            } else {
                format!("{local} = {value};")
            }
        };
        let presence = property.plan.presence;
        let inner_type = types::of(property.inner());

        if let FieldValue::Flag(bit) = property.plan.value {
            let flag = is_set(flags, bit);
            let value = match property.shape() {
                Shape::Always => flag,
                Shape::Present => format!(
                    "{} ? {flag} : (bool?)null",
                    is_set(flags, presence.present.expect("present"))
                ),
                Shape::NonNull => format!(
                    "{} ? {flag} : (bool?)null",
                    is_set(flags, presence.non_null.expect("non-null"))
                ),
                Shape::Both => format!(
                    "{} ? new global::Duplex.Optional<bool?>({} ? {flag} : (bool?)null) : default",
                    is_set(flags, presence.present.expect("present")),
                    is_set(flags, presence.non_null.expect("non-null"))
                ),
            };

            code.line(assign(value));

            return;
        }

        let FieldValue::Value(ty) = &property.plan.value else {
            unreachable!("flattened fields are read in their place");
        };

        if property.shape() == Shape::Always {
            if let Some(value) = self.read_expression(ty) {
                code.line(assign(value));
            } else {
                let read = self.local("value");

                self.read_value(code, ty, &read);
                code.line(assign(read));
            }

            return;
        }

        if declares {
            code.line(format!(
                "{} {local} = {};",
                property.ty(),
                match property.shape() {
                    Shape::Both => "default",
                    _ => "null",
                }
            ));
        }

        let read_into = |code: &mut Code, wrap: &dyn Fn(String) -> String| {
            let read = self.local("value");

            self.read_value(code, ty, &read);
            code.line(format!("{local} = {};", wrap(read)));
        };

        match property.shape() {
            Shape::Present => code.block(
                format!("if ({})", is_set(flags, presence.present.expect("present"))),
                |code| {
                    read_into(code, &|read| read);
                },
            ),
            Shape::NonNull => code.block(
                format!(
                    "if ({})",
                    is_set(flags, presence.non_null.expect("non-null"))
                ),
                |code| {
                    read_into(code, &|read| read);
                },
            ),
            Shape::Both => code.block(
                format!("if ({})", is_set(flags, presence.present.expect("present"))),
                |code| {
                    code.block(
                        format!(
                            "if ({})",
                            is_set(flags, presence.non_null.expect("non-null"))
                        ),
                        |code| {
                            read_into(code, &|read| {
                                format!("new global::Duplex.Optional<{inner_type}?>({read})")
                            });
                        },
                    );
                    code.block("else", |code| {
                        code.line(format!(
                            "{local} = new global::Duplex.Optional<{inner_type}?>(null);"
                        ));
                    });
                },
            ),
            Shape::Always => unreachable!("read above"),
        }
    }

    /// Returns a C# decoding expression for `encoding`, or `None` when statements are required.
    pub(crate) fn read_expression(&self, encoding: &Encoding<'_>) -> Option<String> {
        Some(match encoding {
            Encoding::Bool => format!("{READER}.ReadBool()"),
            Encoding::Char => format!("{READER}.ReadChar()"),
            Encoding::String(requirements) if !requirements.is_any() => {
                format!(
                    "{READER}.ReadValidatedString({})",
                    self.string_arguments(requirements)
                )
            }
            Encoding::String(_) => format!("{READER}.ReadString()"),
            Encoding::Bytes(length) if !length.is_any() => {
                format!("{READER}.ReadValidatedBytes({})", length_arguments(*length))
            }
            Encoding::Bytes(_) => format!("{READER}.ReadBytes()"),
            Encoding::Number(kind, _) => {
                let value = format!("{READER}.Read{}()", types::number_name(*kind));

                match bounds_of(*kind, encoding) {
                    Some(bounds) => range_check(READER, *kind, &value, &bounds),
                    None => value,
                }
            }
            Encoding::Struct(name) | Encoding::Union(name) => {
                format!("{}.Read({READER})", types::pascal(name))
            }
            Encoding::Enumeration(name) => format!("{}.Read({READER})", codec_class(name)),
            Encoding::Nullable(inner) => {
                let value = self.read_expression(inner)?;

                format!(
                    "{READER}.ReadBool() ? ({}?){value} : null",
                    types::of(inner)
                )
            }
            Encoding::List(..) | Encoding::Record(..) => return None,
        })
    }

    /// Writes statements decoding `encoding` into new local `target`.
    pub(crate) fn read_value(&self, code: &mut Code, encoding: &Encoding<'_>, target: &str) {
        if let Some(value) = self.read_expression(encoding) {
            code.line(format!("var {target} = {value};"));

            return;
        }

        match encoding {
            Encoding::List(item, length) => {
                let count = self.local("count");
                let index = self.local("index");

                code.line(format!(
                    "var {target} = new global::System.Collections.Generic.List<{}>();",
                    types::of(item)
                ));
                code.line(format!("var {count} = {};", read_length(*length)));
                self.each_item(
                    code,
                    READER,
                    item,
                    format!("for (var {index} = 0; {index} < {count}; {index}++)"),
                    |code| match self.read_expression(item) {
                        Some(value) => code.line(format!("{target}.Add({value});")),
                        None => {
                            let local = self.local("item");

                            self.read_value(code, item, &local);
                            code.line(format!("{target}.Add({local});"));
                        }
                    },
                );
            }
            Encoding::Record(item, length) => {
                let count = self.local("count");
                let index = self.local("index");
                let key = self.local("key");

                code.line(format!(
                    "var {target} = new global::System.Collections.Generic.Dictionary<string, {}>();",
                    types::of(item)
                ));
                code.line(format!("var {count} = {};", read_length(*length)));
                self.each_item(
                    code,
                    READER,
                    item,
                    format!("for (var {index} = 0; {index} < {count}; {index}++)"),
                    |code| {
                        code.line(format!("var {key} = {READER}.ReadString();"));

                        let local = self.local("item");

                        self.read_value(code, item, &local);
                        code.line(format!("{target}[{key}] = {local};"));
                    },
                );
            }
            Encoding::Nullable(inner) => {
                code.line(format!("{} {target} = null;", types::of(encoding)));
                code.block(format!("if ({READER}.ReadBool())"), |code| {
                    let local = self.local("value");

                    self.read_value(code, inner, &local);
                    code.line(format!("{target} = {local};"));
                });
            }
            _ => unreachable!("the other values are read by an expression"),
        }
    }
}

/// Returns the enumeration codec class name for `name`.
pub(crate) fn codec_class(name: &str) -> String {
    format!("{}Codec", types::pascal(name))
}

fn is_set(flags: &[String], bit: Bit) -> String {
    format!("({} & {}) != 0", flags[bit.byte], bit.mask)
}
