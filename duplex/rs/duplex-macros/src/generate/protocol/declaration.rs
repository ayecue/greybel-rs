use duplex_protocol::{Encoding, Flavour, HealthCheckSetting, Role, SecurityLevel, Status, Symbol};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::ProtocolBuilder;
use crate::generate::names::NameProvider;
use crate::generate::tags::TagsBuilder;

impl<'a> ProtocolBuilder<'a> {
    pub(super) fn generate_protocol(&self) -> TokenStream2 {
        let core = self.core;
        let plan = self.plan.plan;
        let protocol = self.plan.protocol();
        let docs = NameProvider::docs(&protocol.docs);
        let ident = NameProvider::pascal(Symbol::Declared(&protocol.name));
        let channel = &protocol.channel;
        let tags = TagsBuilder::new(core, self.types.definition()).tags(&protocol.tags);
        let channel_id = plan.channel_id;
        let version = Self::version(core, plan.version.major, plan.version.minor);
        let versions = plan.versions.iter().map(|version| {
            let fingerprint = version.fingerprint;
            let status = match version.status {
                Status::Current => quote!(#core::VersionStatus::Current),
                Status::Deprecation(note) => quote!(#core::VersionStatus::Deprecation(#note)),
                Status::Unavailable => quote!(#core::VersionStatus::Unavailable),
            };

            quote! { #core::VersionInfo { fingerprint: #fingerprint, status: #status } }
        });
        let payload = |payload: &Option<Encoding<'_>>, no_payload: TokenStream2| {
            payload
                .as_ref()
                .map_or(no_payload, |payload| self.types.owned_type(payload))
        };
        let flavours = protocol
            .flavours
            .iter()
            .map(|flavour| Self::flavour(core, flavour));
        let handshake = protocol.handshake.is_declared();
        let security = quote::format_ident!(
            "{}",
            match protocol.security.map(|security| security.level) {
                None => "None",
                Some(SecurityLevel::Encryption) => "Encryption",
                Some(SecurityLevel::Authentication) => "Authentication",
            }
        );
        let signed = protocol.security.is_some_and(|security| security.signed);
        let open = payload(&plan.open, quote!(()));
        let ack = payload(&plan.ack, quote!(()));
        let refusal = payload(&plan.refusal, quote!(::std::convert::Infallible));
        let requires = if plan.inbound_messages(Role::Client).next().is_some() {
            quote!(#core::Capabilities::PERSISTENT)
        } else {
            quote!(#core::Capabilities::NONE)
        };
        let core_major = plan.core.major;
        let core_minor = (plan.core.minor > 0).then(|| {
            let minor = plan.core.minor;

            quote!(&& #core::CORE.minor >= #minor)
        });
        let core_message = format!(
            "`{}` builds on the core {}, which this duplex-core doesn't speak",
            protocol.name, plan.core
        );

        quote! {
            const _: () = ::std::assert!(#core::CORE.major == #core_major #core_minor, #core_message);

            #docs
            #[derive(::std::fmt::Debug, ::std::clone::Clone, ::std::marker::Copy)]
            pub struct #ident;

            impl #core::Metadata for #ident {
                const TAGS: &'static [#core::Tag] = #tags;
            }

            impl #core::Protocol for #ident {
                const CHANNEL: &'static str = #channel;
                const CHANNEL_ID: u32 = #channel_id;
                const VERSION: #core::Version = #version;
                const VERSIONS: &'static [#core::VersionInfo] = &[#(#versions),*];
                const FLAVOURS: &'static [#core::TransportFlavour] = &[#(#flavours),*];
                const HANDSHAKE: bool = #handshake;
                const SECURITY: #core::SecurityLevel = #core::SecurityLevel::#security;
                const SIGNED: bool = #signed;
                const REQUIRES: #core::Capabilities = #requires;
                type Open = #open;
                type Ack = #ack;
                type Refusal = #refusal;
            }
        }
    }

    pub(super) fn flavour(core: &TokenStream2, flavour: &Flavour) -> TokenStream2 {
        let transport = &flavour.transport;
        let health_check = match flavour.health_check {
            None => quote!(::std::option::Option::None),
            Some(HealthCheckSetting::Off) => {
                quote!(::std::option::Option::Some(#core::HealthChecks::Off))
            }
            Some(HealthCheckSetting::On {
                interval,
                unhealthy_threshold,
                close_threshold,
            }) => {
                let interval = interval.map(|interval| {
                    let interval = u64::from(interval);

                    quote!(interval: ::std::time::Duration::from_millis(#interval),)
                });
                let unhealthy_threshold = unhealthy_threshold.map(|threshold| {
                    let threshold = threshold as usize;

                    quote!(unhealthy_threshold: #threshold,)
                });
                let close_threshold = close_threshold.map(|threshold| {
                    let threshold = threshold as usize;

                    quote!(close_threshold: #threshold,)
                });

                quote! {
                    ::std::option::Option::Some(#core::HealthChecks::On(#core::HealthCheck {
                        #interval #unhealthy_threshold #close_threshold
                        ..#core::HealthCheck::DEFAULT
                    }))
                }
            }
        };
        let stream_window = match flavour.stream_window {
            Some(window) => quote!(::std::option::Option::Some(#window)),
            None => quote!(::std::option::Option::None),
        };
        let timeout = match flavour.timeout {
            Some(timeout) => {
                let timeout = u64::from(timeout);

                quote!(::std::option::Option::Some(::std::time::Duration::from_millis(#timeout)))
            }
            None => quote!(::std::option::Option::None),
        };
        let max_message = match flavour.max_message {
            Some(max) => quote!(::std::option::Option::Some(#max)),
            None => quote!(::std::option::Option::None),
        };

        quote! {
            #core::TransportFlavour {
                transport_name: #transport,
                health_check: #health_check,
                stream_window: #stream_window,
                timeout: #timeout,
                max_message: #max_message,
            }
        }
    }

    pub(super) fn version(core: &TokenStream2, major: u16, minor: u16) -> TokenStream2 {
        quote!(#core::Version { major: #major, minor: #minor })
    }
}
