use duplex_protocol::{Flavour, HealthCheckSetting, ProtocolPlan, Role, Status, Symbol};
use oxc_ast::ast::{Expression, Statement};

use super::Module;
use crate::ast::{EnumMember, Ts};
use crate::codec::Codec;
use crate::declaration::{HandshakeClass, MessageClass, TagExpressionContainer};

impl Module<'_> {
    pub(super) fn protocol_declarations<'t>(
        ts: &'t Ts<'t>,
        codec: &'t Codec<'t, 't>,
        plan: &ProtocolPlan<'_>,
    ) -> Vec<Statement<'t>> {
        let protocol = plan.protocol;
        let peers = Symbol::Peers(protocol).pascal();
        let docs = ts.doc(format!("Peer role IDs for `{}`.", protocol.name));
        let members = protocol
            .peers()
            .iter()
            .map(|peer| EnumMember {
                span: oxc_span::SPAN,
                name: peer.name.clone(),
                value: u32::from(peer.role.id()),
            })
            .collect();
        let mut statements = vec![ts.enumeration(docs, &peers, members)];

        for message in &plan.messages {
            statements.extend(MessageClass::new(ts, codec, plan, message).declaration());
        }

        let handshake = protocol.handshake.is_declared().then(|| {
            statements.push(HandshakeClass::new(ts, codec, plan).declaration());

            ts.id(&Symbol::Handshake(protocol).pascal())
        });
        let version = |major: u16, minor: u16| {
            ts.object(vec![
                ("major", ts.number_literal(major)),
                ("minor", ts.number_literal(minor)),
            ])
        };
        let versions = plan
            .versions
            .iter()
            .map(|version| {
                let mut properties = vec![("fingerprint", ts.number_literal(version.fingerprint))];

                match version.status {
                    Status::Current => {}
                    Status::Deprecation(note) => {
                        properties.push(("deprecationNote", ts.string_literal(note)));
                    }
                    Status::Unavailable => {
                        properties.push(("unsupported", ts.boolean_literal(true)));
                    }
                }

                ts.object(properties)
            })
            .collect();
        let inbound_messages = Role::ALL
            .map(|role| {
                ts.array_literal(
                    plan.inbound_messages(role)
                        .map(|message| ts.id(&message.message.name))
                        .collect(),
                )
            })
            .into_iter()
            .collect();
        let mut properties = vec![
            ("channel", ts.string_literal(&protocol.channel)),
            ("channelId", ts.number_literal(plan.channel_id)),
            ("version", version(plan.version.major, plan.version.minor)),
            ("core", version(plan.core.major, plan.core.minor)),
            ("versions", ts.array_literal(versions)),
            (
                "peers",
                ts.array_literal(
                    protocol
                        .peers()
                        .iter()
                        .map(|peer| ts.path(&format!("{peers}.{}", peer.name)))
                        .collect(),
                ),
            ),
            ("messages", ts.array_literal(inbound_messages)),
        ];

        properties.extend(handshake.map(|handshake| ("handshake", handshake)));
        properties.push((
            "tags",
            TagExpressionContainer::new(ts, codec.plan().definition).tags(&protocol.tags),
        ));
        if plan.inbound_messages(Role::Client).next().is_some() {
            properties.push(("requiredCapabilities", ts.path("Capabilities.PERSISTENT")));
        }

        if let Some(security) = protocol.security {
            properties.push(("security", ts.string_literal(security.level.keyword())));
            if security.signed {
                properties.push(("signed", ts.boolean_literal(true)));
            }
        }

        if !protocol.flavours.is_empty() {
            let flavours = protocol
                .flavours
                .iter()
                .map(|flavour| (flavour.transport.as_str(), Self::flavour(ts, flavour)))
                .collect();

            properties.push(("flavours", ts.object(flavours)));
        }

        let constant = ts.as_const(ts.object(properties));

        statements.push(ts.exported_const(ts.docs(&protocol.docs), &protocol.name, constant));

        statements
    }

    pub(super) fn flavour<'a>(ts: &Ts<'a>, flavour: &Flavour) -> Expression<'a> {
        let health_check = flavour.health_check.map(|health_check| match health_check {
            HealthCheckSetting::Off => ts.boolean_literal(false),
            HealthCheckSetting::On {
                interval,
                unhealthy_threshold,
                close_threshold,
            } => ts.object(
                [
                    ("interval", interval),
                    ("unhealthyThreshold", unhealthy_threshold),
                    ("closeThreshold", close_threshold),
                ]
                .into_iter()
                .filter_map(|(name, value)| Some((name, ts.number_literal(value?))))
                .collect(),
            ),
        });
        let numbers = [
            ("streamWindow", flavour.stream_window),
            ("timeout", flavour.timeout),
            ("maxMessage", flavour.max_message),
        ]
        .into_iter()
        .filter_map(|(name, value)| Some((name, ts.number_literal(value?))));

        ts.object(
            health_check
                .map(|value| ("healthCheck", value))
                .into_iter()
                .chain(numbers)
                .collect(),
        )
    }
}
