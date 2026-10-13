use std::collections::BTreeSet;

use duplex_protocol::{
    Encoding, MessagePlan, MessageSegment, Plan, Role, RolePlan, Symbol, TypeContent,
};
use oxc_ast::ast::Statement;

use super::{Content, Module};
use crate::ast::{Import, Ts};
use crate::codec::CHANNEL_ERROR;
use crate::declaration::{BODY_KEY, FIELD_TAGS, MEMBER_TAGS, MigrationClass, NEWER, OLDER, TAG};

const MESSAGE: &str = "Message";
/// Core message module exports `Message` and `BODY`.
const MESSAGE_NAMES: [&str; 2] = [MESSAGE, BODY_KEY];

impl Module<'_> {
    pub(super) fn core_imports(&self, plan: &Plan<'_>) -> Vec<Import> {
        let mut core = Vec::new();

        if self.declares_types() || self.declares_protocol() {
            core.extend([Import::value("BinaryReader"), Import::value("BinaryWriter")]);
            if !plan.patterns().is_empty() {
                core.push(Import::value("Pattern"));
            }
        }

        let messages = self.declares_protocol()
            && plan
                .protocol
                .as_ref()
                .is_some_and(|protocol| !protocol.messages.is_empty());
        let declares = |is: fn(&TypeContent<'_>) -> bool| {
            self.declares_types() && plan.types.iter().any(|plan| is(&plan.content))
        };
        let fields = messages
            || declares(|content| {
                matches!(content, TypeContent::Struct(_) | TypeContent::Union(_))
            });
        let members = declares(|content| matches!(content, TypeContent::Enumeration(_)));

        if fields || members {
            core.push(Import::type_only(TAG));
        }

        if fields {
            core.push(Import::value(BODY_KEY));
            core.push(Import::type_only(FIELD_TAGS));
        }

        if members {
            core.push(Import::type_only(MEMBER_TAGS));
        }

        if self.declares_types() && Self::has_gated_items(plan) {
            core.push(Import::value(CHANNEL_ERROR));
        }

        if let Some(protocol) = plan.protocol.as_ref().filter(|_| self.declares_protocol()) {
            let messages = &protocol.messages;

            if !messages.is_empty() {
                core.push(Import::value(MESSAGE));
            }

            let any = |has: fn(&MessagePlan<'_>) -> bool| messages.iter().any(has);

            if any(|message| message.response_value().is_some()) {
                core.push(Import::type_only("ValueResponse"));
            }

            if any(|message| message.response_stream().is_some()) {
                core.push(Import::type_only("StreamResponse"));
            }

            if any(|message| message.stream.is_some()) {
                core.push(Import::type_only("StreamExchange"));
            }

            if protocol.inbound_messages(Role::Client).next().is_some() {
                core.push(Import::value("Capabilities"));
            }
        }

        if let (Content::Role(role), Some(protocol)) = (self.content, &plan.protocol) {
            core.extend([
                Import::value("Channel"),
                Import::value("Router"),
                Import::type_only("ChannelOptions"),
                Import::type_only("Driver"),
                Import::type_only("Transport"),
                Import::type_only("Inbound"),
                Import::type_only("Next"),
                Import::type_only("Reply"),
                Import::type_only("RouteContext"),
                Import::type_only("Routed"),
            ]);

            let inbound_messages: Vec<_> = protocol.role(role).inbound_messages().collect();

            if inbound_messages
                .iter()
                .any(|message| message.stream.is_some())
            {
                core.push(Import::type_only("IncomingStream"));
            }

            if inbound_messages
                .iter()
                .any(|message| message.response_stream().is_some())
            {
                core.push(Import::type_only("ResponseWriter"));
            }
        }

        let values: Vec<String> = core
            .iter()
            .filter(|import| !import.type_only)
            .map(|import| import.name.clone())
            .collect();

        core.retain(|import| !import.type_only || !values.contains(&import.name));

        core
    }

    pub(super) fn has_gated_items(plan: &Plan<'_>) -> bool {
        plan.types.iter().any(|plan| match &plan.content {
            TypeContent::Struct(_) => false,
            TypeContent::Union(variants) => variants.iter().any(|variant| variant.since > 0),
            TypeContent::Enumeration(members) => members.iter().any(|member| member.since > 0),
        })
    }

    pub(super) fn imports<'t>(&self, ts: &Ts<'t>, plan: &Plan<'_>) -> Vec<Statement<'t>> {
        if let Some(migration) = self.migration {
            let core = MigrationClass::imports()
                .into_iter()
                .map(|(name, type_only)| {
                    if type_only {
                        Import::type_only(name)
                    } else {
                        Import::value(name)
                    }
                })
                .collect();

            return vec![
                ts.import(core, self.core),
                ts.import_namespace(OLDER, migration.older_module),
                ts.import_namespace(NEWER, migration.newer_module),
            ];
        }

        let (message, core): (Vec<Import>, Vec<Import>) =
            self.core_imports(plan).into_iter().partition(|import| {
                self.core_message.is_some() && MESSAGE_NAMES.contains(&import.name.as_str())
            });
        let mut imports = match self.core_types {
            Some(core_types) => {
                let (types, values): (Vec<Import>, Vec<Import>) =
                    core.into_iter().partition(|import| import.type_only);

                vec![ts.import(values, self.core), ts.import(types, core_types)]
            }
            None => vec![ts.import(core, self.core)],
        };

        if let Some(core_message) = self.core_message
            && !message.is_empty()
        {
            imports.push(ts.import(message, core_message));
        }

        if let Some(types) = self.types.filter(|_| self.declares_protocol())
            && !plan.types.is_empty()
        {
            let type_names = plan
                .types
                .iter()
                .map(|plan| Import::value(plan.name()))
                .collect();

            imports.push(ts.import(type_names, types));
        }

        if let (Content::Role(role), Some(source), Some(protocol)) =
            (self.content, self.protocol, &plan.protocol)
        {
            imports.push(ts.import(Self::imports_for(protocol.role(role)), source));
            imports.push(ts.export_all(source));
        }

        imports
    }

    pub(super) fn imports_for(plan: RolePlan<'_, '_>) -> Vec<Import> {
        let protocol = plan.protocol();
        let mut type_names = BTreeSet::new();
        let mut names = vec![protocol.name.clone(), Symbol::Peers(protocol).pascal()];
        let handshake = match plan.role {
            Role::Client => vec![&plan.plan.open],
            Role::Provider => vec![&plan.plan.open, &plan.plan.ack],
        };

        for payload in handshake.into_iter().flatten() {
            Self::collect_type_names(payload, &mut type_names);
        }

        for message in plan.inbound_messages() {
            names.push(message.message.name.clone());
            for segment in MessageSegment::ALL {
                if let Some(encoding) = message
                    .segment(segment)
                    .filter(|_| segment != MessageSegment::Failure)
                {
                    Self::collect_type_names(encoding, &mut type_names);
                }
            }
        }

        names.extend(type_names.into_iter().map(str::to_owned));

        names.into_iter().map(Import::value).collect()
    }

    pub(super) fn collect_type_names<'d>(
        encoding: &Encoding<'d>,
        type_names: &mut BTreeSet<&'d str>,
    ) {
        match encoding {
            Encoding::List(item, _) | Encoding::Record(item, _) | Encoding::Nullable(item) => {
                Self::collect_type_names(item, type_names);
            }
            Encoding::Struct(name) | Encoding::Union(name) | Encoding::Enumeration(name) => {
                type_names.insert(name);
            }
            Encoding::Bool
            | Encoding::Char
            | Encoding::String(_)
            | Encoding::Bytes(_)
            | Encoding::Number(..) => {}
        }
    }
}
