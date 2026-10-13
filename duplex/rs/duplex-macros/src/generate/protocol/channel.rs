use duplex_protocol::{Role, Symbol};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::ProtocolBuilder;
use crate::generate::names::NameProvider;

impl<'a> ProtocolBuilder<'a> {
    /// Generates a driver-erased channel trait for messages sent by `role`, so callers can use `dyn {Peer}Channel`
    /// without naming the driver type.
    pub(super) fn generate_channel(&self, role: Role) -> TokenStream2 {
        let core = self.core;
        let local = self.plan.protocol().peer(role);
        let peer = NameProvider::pascal(Symbol::Declared(&local.name));
        let channel = NameProvider::pascal(Symbol::Channel(local));
        let messages: Vec<_> = self
            .plan
            .plan
            .inbound_messages(role.remote())
            .map(|message| {
                let method = NameProvider::snake(Symbol::Send(message.message));
                let body = Self::body(message, &quote!('_));
                let static_body = Self::body(message, &quote!('static));
                let docs = format!(" Sends `{}`.", message.message.name);
                let handle = quote! {
                    ::std::result::Result<
                        <<#static_body as #core::Message>::Exchange as #core::Exchange>::Handle,
                        #core::Error,
                    >
                };

                (method, body, handle, docs)
            })
            .collect();
        let declarations = messages.iter().map(|(method, body, handle, docs)| {
            quote! {
                #[doc = #docs]
                fn #method(&self, message: &#body) -> #handle;
            }
        });
        let implementations = messages.iter().map(|(method, body, handle, _)| {
            quote! {
                fn #method(&self, message: &#body) -> #handle {
                    self.send(message)
                }
            }
        });
        let docs = format!(
            " The messages the {} `{}` sends, over a channel of any driver.",
            role.keyword(),
            local.name
        );

        quote! {
            #[doc = #docs]
            pub trait #channel {
                #(#declarations)*
            }

            impl<D: #core::Driver> #channel for #core::Channel<#peer, D> {
                #(#implementations)*
            }
        }
    }
}
