//! C# declaration generator for protocol types, messages, and provider APIs.

use duplex_protocol::{
    DocumentationRef, Encoding, FieldsPlan, HealthCheckSetting, MemberPlan, MessagePlan,
    PayloadPlan, Plan, ProtocolPlan, Role, SecurityLevel, Status, Symbol, TypeContent, TypePlan,
    VariantPlan,
};

use crate::code::Code;
use crate::codec::{Codec, Property, codec_class};
use crate::tags::TagWriter;
use crate::types;

const WRITER_PARAMETER: &str = "global::Duplex.DuplexWriter writer";
const READER_PARAMETER: &str = "global::Duplex.DuplexReader reader";

pub(crate) struct DeclarationBuilder<'p, 'd> {
    plan: &'p Plan<'d>,
    codec: Codec<'p, 'd>,
    tags: TagWriter<'p, 'd>,
}

impl<'p, 'd> DeclarationBuilder<'p, 'd> {
    pub(crate) fn new(plan: &'p Plan<'d>) -> Self {
        Self {
            plan,
            codec: Codec::new(plan),
            tags: TagWriter::new(plan),
        }
    }

    pub(crate) fn patterns(&self, code: &mut Code) {
        self.codec.patterns(code);
    }

    pub(crate) fn constants(&self, code: &mut Code) {
        if self.plan.constants.is_empty() {
            return;
        }

        code.block("public static partial class Constants", |code| {
            for constant in &self.plan.constants {
                code.docs(docs(&constant.constant.docs));

                let ty = types::of(&constant.ty);
                let modifier = if matches!(
                    constant.ty,
                    Encoding::Number(..) | Encoding::Bool | Encoding::Char | Encoding::String(_)
                ) {
                    "const"
                } else {
                    "static readonly"
                };

                code.line(format!(
                    "public {modifier} {ty} {} = {};",
                    types::pascal(&constant.constant.name),
                    types::literal(&constant.value, &constant.ty)
                ));
            }
        });
    }

    pub(crate) fn types(&self, code: &mut Code) {
        for plan in &self.plan.types {
            code.line("");

            match &plan.content {
                TypeContent::Struct(fields) => self.declare_struct(code, plan, fields),
                TypeContent::Union(variants) => self.declare_union(code, plan, variants),
                TypeContent::Enumeration(members) => self.declare_enumeration(code, plan, members),
            }
        }
    }

    fn declare_struct(&self, code: &mut Code, plan: &TypePlan<'_>, fields: &FieldsPlan<'_>) {
        let name = types::pascal(&plan.declaration.name);

        code.docs(docs(&plan.declaration.docs));
        code.block(format!("public sealed class {name}"), |code| {
            Self::arguments(
                code,
                "public static readonly global::Duplex.StructType StructType = new global::Duplex.StructType(",
                vec![
                    types::string(&plan.declaration.name),
                    self.tags.tags(&plan.declaration.tags),
                    self.tags.fields(&name, fields),
                    TagWriter::create(&name, fields),
                ],
            );
            code.line("");
            Self::declare_fields(code, &name, fields);
            code.line("");
            code.block(format!("public void Write({WRITER_PARAMETER})"), |code| {
                self.codec.reset_locals();
                self.codec.write_fields(code, fields, &name, "this");
            });
            code.line("");
            code.block(
                format!("public static {name} Read({READER_PARAMETER})"),
                |code| {
                    self.codec.reset_locals();

                    let locals = self.codec.read_fields(code, fields, &name);

                    code.line(format!("return new {name}({});", locals.join(", ")));
                },
            );
        });
    }

    /// Writes the field constructor and properties for `owner`.
    fn declare_fields(code: &mut Code, owner: &str, fields: &FieldsPlan<'_>) {
        let properties: Vec<Property<'_, '_>> = fields
            .properties()
            .into_iter()
            .map(|field| Property::of(field, owner))
            .collect();
        let trailing_optional = properties
            .iter()
            .rev()
            .take_while(|property| property.plan.can_be_left_out())
            .count();
        let optional_start = properties.len() - trailing_optional;
        let parameters: Vec<String> = properties
            .iter()
            .enumerate()
            .map(|(index, property)| {
                if index < optional_start {
                    format!("{} {}", property.ty(), property.parameter)
                } else {
                    format!(
                        "{} {} = {}",
                        property.ty(),
                        property.parameter,
                        Self::parameter_default(property)
                    )
                }
            })
            .collect();
        let head = format!("public {owner}({})", parameters.join(", "));

        code.block(head, |code| {
            for property in &properties {
                code.line(format!("{} = {};", property.name, property.parameter));
            }
        });
        for property in &properties {
            code.line("");
            code.docs(docs(&property.plan.field.docs));
            code.line(format!(
                "public {} {} {{ get; }}",
                property.ty(),
                property.name
            ));
        }
    }

    /// Returns a constructor parameter's default compile-time constant.
    fn parameter_default(property: &Property<'_, '_>) -> String {
        match (&property.plan.default, property.shape()) {
            (Some(value), shape) if shape != crate::codec::Shape::Both => {
                types::literal(value, &property.plan.ty)
            }
            _ => "default".to_owned(),
        }
    }

    fn declare_union(&self, code: &mut Code, plan: &TypePlan<'_>, variants: &[VariantPlan<'_>]) {
        let name = types::pascal(&plan.declaration.name);

        code.docs(docs(&plan.declaration.docs));
        code.block(format!("public abstract class {name}"), |code| {
            let variant_types: Vec<String> = variants
                .iter()
                .map(|variant| {
                    let variant_name = types::pascal(&variant.variant.name);
                    let fields = match &variant.payload {
                        PayloadPlan::Unit => TagWriter::no_fields().to_owned(),
                        PayloadPlan::Value(encoding) => TagWriter::value_fields(&variant_name, encoding),
                        PayloadPlan::Fields(fields) => self.tags.fields(&variant_name, fields),
                    };

                    format!(
                        "new global::Duplex.VariantType({}, typeof({variant_name}), {}, {fields})",
                        types::string(&variant.variant.name),
                        self.tags.tags(&variant.variant.tags)
                    )
                })
                .collect();

            Self::arguments(
                code,
                "public static readonly global::Duplex.UnionType UnionType = new global::Duplex.UnionType(",
                vec![
                    types::string(&plan.declaration.name),
                    self.tags.tags(&plan.declaration.tags),
                    format!("new global::Duplex.VariantType[] {{ {} }}", variant_types.join(", ")),
                ],
            );
            code.line("");
            code.line(format!("private protected {name}() {{ }}"));
            code.line("");
            code.line(format!("public abstract void Write({WRITER_PARAMETER});"));
            code.line("");
            code.block(format!("public static {name} Read({READER_PARAMETER})"), |code| {
                code.line("var index = reader.ReadU32();");
                code.block("switch (index)", |code| {
                    for variant in variants {
                        code.line(format!(
                            "case {}: return {}.Read(reader);",
                            variant.index,
                            types::pascal(&variant.variant.name)
                        ));
                    }

                    code.line(format!(
                        "default: throw new global::Duplex.MalformedException($\"no variant {{index}} of `{name}`\");"
                    ));
                });
            });
            for variant in variants {
                code.line("");
                self.declare_variant(code, &name, variant);
            }
        });
    }

    fn declare_variant(&self, code: &mut Code, union: &str, variant: &VariantPlan<'_>) {
        let name = types::pascal(&variant.variant.name);
        let payload_types: Vec<String> = match &variant.payload {
            PayloadPlan::Unit => Vec::new(),
            PayloadPlan::Value(encoding) => vec![types::of(encoding)],
            PayloadPlan::Fields(fields) => {
                fields.fields().map(|field| types::of(&field.ty)).collect()
            }
        };

        assert!(
            !payload_types.iter().any(|ty| ty
                .split(|character: char| !character.is_alphanumeric() && character != '_')
                .any(|part| part == name)),
            "the variant `{name}` of `{union}` has the name of a type its payload names, which C# can't refer to \
             inside the variant's class; give one of them another name",
        );

        code.docs(docs(&variant.variant.docs));
        code.block(format!("public sealed class {name} : {union}"), |code| {
            match &variant.payload {
                PayloadPlan::Unit => code.line(format!("public {name}() {{ }}")),
                PayloadPlan::Value(encoding) => {
                    code.block(
                        format!("public {name}({} value)", types::of(encoding)),
                        |code| {
                            code.line("Value = value;");
                        },
                    );
                    code.line("");
                    code.line(format!("public {} Value {{ get; }}", types::of(encoding)));
                }
                PayloadPlan::Fields(fields) => Self::declare_fields(code, &name, fields),
            }

            code.line("");
            code.block(
                format!("public override void Write({WRITER_PARAMETER})"),
                |code| {
                    self.codec.reset_locals();
                    Self::version_guard(code, variant.since);
                    code.line(format!("writer.WriteU32({});", variant.index));

                    match &variant.payload {
                        PayloadPlan::Unit => {}
                        PayloadPlan::Value(encoding) => {
                            self.codec.write_value(code, encoding, "this.Value");
                        }
                        PayloadPlan::Fields(fields) => {
                            self.codec.write_fields(code, fields, &name, "this");
                        }
                    }
                },
            );
            code.line("");
            code.block(
                format!("public static new {name} Read({READER_PARAMETER})"),
                |code| {
                    self.codec.reset_locals();

                    match &variant.payload {
                        PayloadPlan::Unit => code.line(format!("return new {name}();")),
                        PayloadPlan::Value(encoding) => {
                            self.codec.read_value(code, encoding, "value");
                            code.line(format!("return new {name}(value);"));
                        }
                        PayloadPlan::Fields(fields) => {
                            let locals = self.codec.read_fields(code, fields, &name);

                            code.line(format!("return new {name}({});", locals.join(", ")));
                        }
                    }
                },
            );
        });
    }

    fn declare_enumeration(
        &self,
        code: &mut Code,
        plan: &TypePlan<'_>,
        members: &[MemberPlan<'_>],
    ) {
        let name = types::pascal(&plan.declaration.name);

        code.docs(docs(&plan.declaration.docs));
        code.block(format!("public enum {name} : uint"), |code| {
            for member in members {
                code.docs(docs(&member.member.docs));
                code.line(format!(
                    "{} = {},",
                    types::pascal(&member.member.name),
                    member.value
                ));
            }
        });
        code.line("");
        code.docs(&[format!("Serializes and deserializes `{name}`.")]);
        code.block(format!("public static class {}", codec_class(&plan.declaration.name)), |code| {
            let member_types: Vec<String> = members
                .iter()
                .map(|member| {
                    format!(
                        "new global::Duplex.MemberType({}, {}u, {})",
                        types::string(&member.member.name),
                        member.value,
                        self.tags.tags(&member.member.tags)
                    )
                })
                .collect();

            Self::arguments(
                code,
                "public static readonly global::Duplex.EnumType EnumType = new global::Duplex.EnumType(",
                vec![
                    types::string(&plan.declaration.name),
                    self.tags.tags(&plan.declaration.tags),
                    if member_types.is_empty() {
                        "global::System.Array.Empty<global::Duplex.MemberType>()".to_owned()
                    } else {
                        format!("new global::Duplex.MemberType[] {{ {} }}", member_types.join(", "))
                    },
                ],
            );
            code.line("");
            code.block(format!("public static void Write({WRITER_PARAMETER}, {name} value)"), |code| {
                for member in members.iter().filter(|member| member.since > 0) {
                    code.block(
                        format!(
                            "if (value == {name}.{} && writer.Minor < {})",
                            types::pascal(&member.member.name),
                            member.since
                        ),
                        |code| code.line("throw global::Duplex.ChannelException.CreateUnavailable();"),
                    );
                }

                code.line("writer.WriteU32((uint)value);");
            });
            code.line("");
            code.block(format!("public static {name} Read({READER_PARAMETER})"), |code| {
                code.line("var value = reader.ReadU32();");
                code.block("switch (value)", |code| {
                    let cases: Vec<String> = members.iter().map(|member| format!("case {}:", member.value)).collect();

                    code.line(format!("{} return ({name})value;", cases.join(" ")));
                    code.line(format!(
                        "default: throw new global::Duplex.MalformedException($\"no member {{value}} of `{name}`\");"
                    ));
                });
            });
        });
    }

    fn version_guard(code: &mut Code, since: u16) {
        if since > 0 {
            code.block(format!("if (writer.Minor < {since})"), |code| {
                code.line("throw global::Duplex.ChannelException.CreateUnavailable();");
            });
        }
    }

    pub(crate) fn messages(&self, code: &mut Code, protocol: &ProtocolPlan<'_>) {
        for message in &protocol.messages {
            code.line("");
            self.declare_message(code, message);
        }
    }

    fn declare_message(&self, code: &mut Code, message: &MessagePlan<'_>) {
        let name = types::pascal(&message.message.name);

        code.docs(docs(&message.message.docs));
        code.block(format!("public sealed class {name} : global::Duplex.Message"), |code| {
            Self::arguments(
                code,
                "public static readonly global::Duplex.MessageType MessageType = new global::Duplex.MessageType(",
                vec![
                    types::string(&message.message.name),
                    format!("id: {}", message.id),
                    format!("since: {}", message.since),
                    format!("sender: global::Duplex.Role.{}", role_name(message.sender.role)),
                    format!("limit: {}", message.limit),
                    format!("responds: {}", message.has_response()),
                    format!("streams: {}", message.stream.is_some() || message.response_stream().is_some()),
                    "read: reader => Read(reader)".to_owned(),
                    format!("tags: {}", self.tags.tags(&message.message.tags)),
                    format!("fields: {}", self.tags.fields(&name, &message.body)),
                    format!("create: {}", TagWriter::create(&name, &message.body)),
                ],
            );
            code.line("");
            Self::declare_fields(code, &name, &message.body);
            code.line("");
            code.line("public override global::Duplex.MessageType GetMessageType() => MessageType;");
            code.line("");
            code.block(format!("public override void Write({WRITER_PARAMETER})"), |code| {
                self.codec.reset_locals();
                self.codec.write_fields(code, &message.body, &name, "this");
            });
            code.line("");
            code.block(format!("public static {name} Read({READER_PARAMETER})"), |code| {
                self.codec.reset_locals();

                let locals = self.codec.read_fields(code, &message.body, &name);

                code.line(format!("return new {name}({});", locals.join(", ")));
            });
        });
    }

    /// Writes `head`, one indented line per argument, and `);` after the final argument.
    fn arguments(code: &mut Code, head: &str, arguments: Vec<String>) {
        code.line(head);

        let count = arguments.len();

        for (index, argument) in arguments.into_iter().enumerate() {
            let end = if index + 1 == count { ");" } else { "," };

            code.line(format!("    {argument}{end}"));
        }
    }

    pub(crate) fn protocol(&self, code: &mut Code, protocol: &ProtocolPlan<'_>) {
        let definition = protocol.protocol;
        let name = types::pascal(&definition.name);

        code.line("");
        code.docs(docs(&definition.docs));
        code.block(format!("public static class {name}"), |code| {
            code.line("public static readonly global::Duplex.ProtocolDefinition Definition = new global::Duplex.ProtocolDefinition(");

            let mut arguments = vec![
                format!("channel: {}", types::string(&definition.channel)),
                format!("channelId: {}u", protocol.channel_id),
                format!("version: new global::Duplex.VersionNumber({}, {})", protocol.version.major, protocol.version.minor),
                format!("core: new global::Duplex.VersionNumber({}, {})", protocol.core.major, protocol.core.minor),
            ];
            let versions: Vec<String> = protocol
                .versions
                .iter()
                .map(|version| {
                    let (note, unavailable) = match version.status {
                        Status::Current => ("null".to_owned(), false),
                        Status::Deprecation(note) => (types::string(note), false),
                        Status::Unavailable => ("null".to_owned(), true),
                    };

                    format!("new global::Duplex.VersionInfo({}u, {note}, {unavailable})", version.fingerprint)
                })
                .collect();

            arguments.push(format!("versions: new[] {{ {} }}", versions.join(", ")));

            let inbound = |role: Role| {
                let messages: Vec<String> = protocol
                    .inbound_messages(role)
                    .map(|message| format!("{}.MessageType", types::pascal(&message.message.name)))
                    .collect();

                if messages.is_empty() {
                    "global::System.Array.Empty<global::Duplex.MessageType>()".to_owned()
                } else {
                    format!("new[] {{ {} }}", messages.join(", "))
                }
            };

            arguments.push(format!(
                "messages: new[] {{ {}, {} }}",
                inbound(Role::Client),
                inbound(Role::Provider)
            ));

            let security = definition.security.as_ref().map_or_else(
                || "null".to_owned(),
                |security| match security.level {
                    SecurityLevel::Encryption => "global::Duplex.SecurityLevel.Encryption".to_owned(),
                    SecurityLevel::Authentication => "global::Duplex.SecurityLevel.Authentication".to_owned(),
                },
            );

            arguments.push(format!("security: {security}"));

            let flavours: Vec<String> = definition
                .flavours
                .iter()
                .map(|flavour| {
                    let health_check = match flavour.health_check {
                        None | Some(HealthCheckSetting::Off) => "null".to_owned(),
                        Some(HealthCheckSetting::On { interval, unhealthy_threshold, close_threshold }) => format!(
                            "global::Duplex.HealthCheck.Default.With({}, {}, {})",
                            optional(interval),
                            optional(unhealthy_threshold),
                            optional(close_threshold)
                        ),
                    };

                    format!(
                        "new global::Duplex.Flavour({}, healthCheck: {health_check}, maxMessage: {}, turnsOffHealthCheck: {})",
                        types::string(&flavour.transport),
                        optional(flavour.max_message),
                        flavour.health_check == Some(HealthCheckSetting::Off)
                    )
                })
                .collect();

            arguments.push(if flavours.is_empty() {
                "flavours: global::System.Array.Empty<global::Duplex.Flavour>()".to_owned()
            } else {
                format!("flavours: new[] {{ {} }}", flavours.join(", "))
            });
            arguments.push(format!("tags: {}", self.tags.tags(&definition.tags)));

            let count = arguments.len();

            for (index, argument) in arguments.into_iter().enumerate() {
                let end = if index + 1 == count { ");" } else { "," };

                code.line(format!("    {argument}{end}"));
            }
        });
    }

    /// Writes provider notification router and channel code with a `Send` method per outgoing notification.
    pub(crate) fn provider(code: &mut Code, protocol: &ProtocolPlan<'_>) {
        let definition = protocol.protocol;
        let protocol_name = types::pascal(&definition.name);
        let peer = definition.peer(Role::Provider);
        let router = Symbol::Router(peer).pascal();
        let channel = Symbol::Channel(peer).pascal();
        let notifications = |role: Role| {
            protocol.messages.iter().filter(move |message| {
                message.message.receiver == role
                    && !message.has_response()
                    && message.stream.is_none()
            })
        };

        code.line("");
        code.docs(&[format!(
            "Routes notifications received by provider `{}`. Override a message method to handle it.",
            peer.name
        )]);
        code.block(
            format!("public abstract class {router} : global::Duplex.Router"),
            |code| {
                for message in notifications(Role::Provider) {
                    let name = types::pascal(&message.message.name);

                    code.line(format!(
                        "public virtual void {}({name} message) {{ }}",
                        Symbol::Handler(message.message).pascal()
                    ));
                    code.line("");
                }

                code.block(
                    "public override void Notify(global::Duplex.Message message)",
                    |code| {
                        code.block("switch (message)", |code| {
                            for message in notifications(Role::Provider) {
                                let name = types::pascal(&message.message.name);

                                code.line(format!(
                                    "case {name} received: {}(received); return;",
                                    Symbol::Handler(message.message).pascal()
                                ));
                            }
                        });
                    },
                );
            },
        );
        code.line("");
        code.docs(&[format!(
            "The provider `{}` for `{}` listens for its client over a transport.",
            peer.name, definition.name
        )]);
        code.block(format!("public sealed class {channel} : global::Duplex.ProviderChannel"), |code| {
            code.line(format!(
                "public {channel}(global::Duplex.Transport transport, {router} router, global::Duplex.ChannelOptions? options = null) : base(transport, {protocol_name}.Definition, router, options) {{ }}"
            ));
            for message in notifications(Role::Client) {
                code.line("");
                code.line(format!(
                    "public void Send({} message) => base.Send(message);",
                    types::pascal(&message.message.name)
                ));
            }
        });
    }
}

fn docs(docs: &DocumentationRef) -> &[String] {
    docs.lines()
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Client => "Client",
        Role::Provider => "Provider",
    }
}

fn optional(value: Option<u32>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}
