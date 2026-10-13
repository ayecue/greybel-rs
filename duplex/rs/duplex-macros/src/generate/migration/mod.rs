mod conversion;

use std::cell::RefCell;
use std::collections::BTreeMap;

use duplex_protocol::{
    Direction, Encoding, MessageMigration, MessagePlan, MessageSegment, MigrationPlan, Plan,
    ProtocolPlan, Rewrite, Role, Symbol,
};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Ident, Path};

use super::names::NameProvider;
use super::rust_type::RustTypeMapper;

pub struct Side<'a> {
    plan: &'a Plan<'a>,
    protocol: &'a ProtocolPlan<'a>,
    path: &'a Path,
    types: RustTypeMapper<'a>,
}

impl<'a> Side<'a> {
    pub fn new(core: &'a TokenStream2, plan: &'a Plan<'a>, path: &'a Path) -> Option<Self> {
        Some(Self {
            plan,
            protocol: plan.protocol.as_ref()?,
            path,
            types: RustTypeMapper::new(core, plan, Some(path)),
        })
    }

    fn peer(&self, role: Role) -> TokenStream2 {
        let path = self.path;
        let peer = NameProvider::pascal(Symbol::Declared(&self.protocol.protocol.peer(role).name));

        quote!(#path::#peer)
    }
}

pub struct MigrationBuilder<'a> {
    core: &'a TokenStream2,
    plan: MigrationPlan<'a, 'a>,
    older: Side<'a>,
    newer: Side<'a>,
    conversions: RefCell<BTreeMap<(Direction, String, String), TokenStream2>>,
}

struct Rewritten<'p, 'd> {
    migration: &'p MessageMigration<'p, 'd>,
    segment: MessageSegment,
    direction: Direction,
    migrate: Option<Rewrite<&'p Encoding<'d>>>,
}

impl<'a> MigrationBuilder<'a> {
    pub fn new(
        core: &'a TokenStream2,
        plan: MigrationPlan<'a, 'a>,
        older: Side<'a>,
        newer: Side<'a>,
    ) -> Self {
        Self {
            core,
            plan,
            older,
            newer,
            conversions: RefCell::default(),
        }
    }

    pub fn generate(&self) -> TokenStream2 {
        let core = self.core;
        let protocol = self.newer.protocol.protocol;
        let from = self.older.protocol.version.major;
        let name = Symbol::Migration(protocol, from).pascal();
        let trait_ident = NameProvider::ident(&name);
        let migrate = format_ident!("{name}Rewrite");
        let conversions_ident = format_ident!("{name}Conversions");
        let rewritten = self.rewritten();
        let methods: Vec<TokenStream2> = rewritten
            .iter()
            .map(|rewritten| self.method(rewritten, &conversions_ident))
            .collect();
        let migration_cases: Vec<TokenStream2> = rewritten
            .iter()
            .map(|rewritten| self.rewrite_case(rewritten))
            .chain(self.passthrough())
            .collect();
        let provider = |side: &Side<'_>| side.peer(Role::Provider);
        let (older_provider, newer_provider) = (provider(&self.older), provider(&self.newer));
        let older_messages = self.plan.messages.iter().map(|migration| {
            let role = Self::role(core, migration.older.message.receiver);
            let (older, newer) = (migration.older.id, migration.newer.id);

            quote! { (#role, #newer) => ::std::option::Option::Some(#older), }
        });
        let conversions = self.conversions.borrow();
        let conversions = conversions.values();
        let docs = format!(
            " Rewrites what clients of `{}` {from}.x send and are sent, up to {}.x.",
            protocol.name,
            from + 1
        );

        quote! {
            #[doc = #docs]
            pub trait #trait_ident: 'static {
                #(#methods)*

                fn attach<D: #core::Driver>(self, channel: &#core::Channel<#newer_provider, D>)
                where
                    Self: Sized,
                {
                    channel.migrate(#migrate(self));
                }
            }

            struct #migrate<M>(M);

            struct #conversions_ident;

            impl #conversions_ident {
                #(#conversions)*
            }

            #[allow(unused_variables, unreachable_code, unreachable_patterns, clippy::match_single_binding)]
            impl<M: #trait_ident> #core::Migration for #migrate<M> {
                type Older = #older_provider;
                type Newer = #newer_provider;

                fn migrate(
                    &self,
                    segment: #core::MessageSegment,
                    receiver: #core::Role,
                    type_id: u32,
                    reader: &mut #core::BinaryReader<'_>,
                    writer: &mut #core::BinaryWriter,
                ) -> ::std::result::Result<u32, #core::ErrorCode> {
                    match (receiver, segment, type_id) {
                        #(#migration_cases)*
                        _ => ::std::result::Result::Err(#core::ErrorCode::UnsupportedVersion),
                    }
                }

                fn older_message(&self, receiver: #core::Role, type_id: u32) -> ::std::option::Option<u32> {
                    match (receiver, type_id) {
                        #(#older_messages)*
                        _ => ::std::option::Option::None,
                    }
                }
            }
        }
    }

    fn passthrough(&self) -> Vec<TokenStream2> {
        let core = self.core;

        self.plan
            .messages
            .iter()
            .flat_map(|migration| {
                let role = Self::role(core, migration.older.message.receiver);

                migration.passthrough.iter().map(move |&segment| {
                    let up = Self::goes_up(migration, segment);
                    let (from_id, to_id) = if up {
                        (migration.older.id, migration.newer.id)
                    } else {
                        (migration.newer.id, migration.older.id)
                    };
                    let segment = Self::segment(core, segment);

                    quote! {
                        (#role, #segment, #from_id) => ::std::result::Result::Ok(#to_id),
                    }
                })
            })
            .collect()
    }

    fn rewritten(&self) -> Vec<Rewritten<'_, 'a>> {
        self.plan
            .messages
            .iter()
            .flat_map(|migration| {
                let message = Rewritten {
                    migration,
                    segment: MessageSegment::Message,
                    direction: migration.message.direction,
                    migrate: None,
                };
                let segments = migration
                    .segments
                    .iter()
                    .map(move |(segment, migrate)| Rewritten {
                        migration,
                        segment: *segment,
                        direction: migrate.direction,
                        migrate: Some(*migrate),
                    });

                std::iter::once(message).chain(segments)
            })
            .collect()
    }

    fn goes_up(migration: &MessageMigration<'_, '_>, segment: MessageSegment) -> bool {
        let up = migration.message.direction == Direction::Up;

        up != segment.is_response()
    }

    fn role(core: &TokenStream2, role: Role) -> TokenStream2 {
        match role {
            Role::Client => quote!(#core::Role::Client),
            Role::Provider => quote!(#core::Role::Provider),
        }
    }

    fn segment(core: &TokenStream2, segment: MessageSegment) -> TokenStream2 {
        let variant = format_ident!("{}", segment.pascal());

        quote!(#core::MessageSegment::#variant)
    }

    fn side(&self, newer: bool) -> &Side<'a> {
        if newer { &self.newer } else { &self.older }
    }

    fn message_type(side: &Side<'_>, plan: &MessagePlan<'_>) -> TokenStream2 {
        let path = side.path;
        let ident = NameProvider::pascal(Symbol::Declared(&plan.message.name));

        if RustTypeMapper::borrows(&plan.body) {
            quote!(#path::#ident<'static>)
        } else {
            quote!(#path::#ident)
        }
    }

    fn types(&self, rewritten: &Rewritten<'_, '_>) -> (TokenStream2, TokenStream2) {
        let (from, to) = (
            self.side(rewritten.direction.from_newer()),
            self.side(!rewritten.direction.from_newer()),
        );

        match rewritten.migrate {
            None => {
                let message = rewritten.migration.message;

                (
                    Self::message_type(from, message.from),
                    Self::message_type(to, message.to),
                )
            }
            Some(migrate) => (
                from.types.owned_type(migrate.from),
                to.types.owned_type(migrate.to),
            ),
        }
    }

    fn rewrite_case(&self, rewritten: &Rewritten<'_, '_>) -> TokenStream2 {
        let core = self.core;
        let migration = rewritten.migration;
        let role = Self::role(core, migration.older.message.receiver);
        let segment = Self::segment(core, rewritten.segment);
        let (from_id, to_id) = if rewritten.direction.from_newer() {
            (migration.newer.id, migration.older.id)
        } else {
            (migration.older.id, migration.newer.id)
        };
        let (from_type, _) = self.types(rewritten);
        let method =
            NameProvider::snake(Symbol::Migrate(migration.newer.message, rewritten.segment));

        quote! {
            (#role, #segment, #from_id) => {
                let value = <#from_type as #core::BinaryDecode>::read(reader)
                    .map_err(|_| #core::ErrorCode::MalformedMessage)?;
                let value = self.0.#method(value)?;
                #core::BinaryEncode::write(&value, writer)
                    .map_err(|error| #core::ErrorCode::of_migration(&error))?;
                ::std::result::Result::Ok(#to_id)
            }
        }
    }

    fn method(&self, rewritten: &Rewritten<'_, '_>, conversions: &Ident) -> TokenStream2 {
        let core = self.core;
        let migration = rewritten.migration;
        let name = &migration.older.message.name;
        let method =
            NameProvider::snake(Symbol::Migrate(migration.newer.message, rewritten.segment));
        let (from_type, to_type) = self.types(rewritten);
        let docs = match rewritten.segment {
            MessageSegment::Message => format!(" Rewrites `{name}`."),
            MessageSegment::MessageItem => format!(" Rewrites an item of the stream of `{name}`."),
            MessageSegment::MessageEnd => {
                format!(" Rewrites what the stream of `{name}` ends with.")
            }
            MessageSegment::ResponseEnd => format!(" Rewrites the response to `{name}`."),
            MessageSegment::ResponseStart => {
                format!(" Rewrites what the response to `{name}` starts with.")
            }
            MessageSegment::ResponseItem => {
                format!(" Rewrites an item of the response to `{name}`.")
            }
            MessageSegment::Failure => format!(" Rewrites what `{name}` failed with."),
        };
        let (body, parameter) = match rewritten.migrate {
            None => {
                let message = migration.message;
                let body = message.by_default.then(|| {
                    let fields = self.convert_fields(
                        message.direction,
                        &message.from.body,
                        &message.to.body,
                        &|field| quote!(message.#field),
                        true,
                        conversions,
                    );
                    let to_path = self.side(!message.direction.from_newer()).path;
                    let ident = NameProvider::pascal(Symbol::Declared(&message.to.message.name));

                    quote! {{ ::std::result::Result::Ok(#to_path::#ident { #fields }) }}
                });
                let parameter = if body.is_some() && message.from.body.is_empty() {
                    quote!(_message)
                } else {
                    quote!(message)
                };

                (body, parameter)
            }
            Some(migrate) => {
                let body = migrate.by_default.then(|| {
                    let value = self.convert(
                        migrate.direction,
                        migrate.from,
                        migrate.to,
                        &quote!(value),
                        false,
                        conversions,
                    );

                    quote! {{ ::std::result::Result::Ok(#value) }}
                });

                (body, quote!(value))
            }
        };
        let terminator = body.is_none().then(|| quote!(;));

        quote! {
            #[doc = #docs]
            fn #method(&self, #parameter: #from_type) -> ::std::result::Result<#to_type, #core::ErrorCode>
            #body
            #terminator
        }
    }
}
