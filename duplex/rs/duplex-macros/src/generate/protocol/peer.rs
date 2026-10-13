use duplex_protocol::{Role, Symbol};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::ProtocolBuilder;
use crate::generate::names::NameProvider;

impl<'a> ProtocolBuilder<'a> {
    pub(super) fn generate_peer(&self, role: Role) -> TokenStream2 {
        let core = self.core;
        let protocol = self.plan.protocol();
        let peer = protocol.peer(role);
        let docs = NameProvider::docs(&peer.docs);
        let ident = NameProvider::pascal(Symbol::Declared(&peer.name));
        let protocol_ident = NameProvider::pascal(Symbol::Declared(&protocol.name));
        let remote = NameProvider::pascal(Symbol::Declared(&protocol.peer(role.remote()).name));
        let (role, open_type) = match role {
            Role::Client => (
                quote!(#core::Role::Client),
                quote!(<#protocol_ident as #core::Protocol>::Open),
            ),
            Role::Provider => (quote!(#core::Role::Provider), quote!(())),
        };
        let (sends, timeouts) = self.sends(peer.role);

        quote! {
            #docs
            #[derive(::std::fmt::Debug, ::std::clone::Clone, ::std::marker::Copy)]
            pub struct #ident;

            impl #core::Peer for #ident {
                type Protocol = #protocol_ident;
                type Remote = #remote;
                const ROLE: #core::Role = #role;
                type OpenPayload = #open_type;
                const SEND_REQUIREMENTS: &'static [&'static #core::MessageRequirementTable] = &[#(#sends),*];
                const TIMEOUTS: &'static [::std::option::Option<::std::time::Duration>] = &[#(#timeouts),*];
            }
        }
    }

    /// Returns requirement and timeout expressions for `role` messages, indexed by message type ID.
    fn sends(&self, role: Role) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
        let core = self.core;
        let outbound_messages: Vec<_> = self
            .plan
            .plan
            .messages
            .iter()
            .filter(|message| message.sender.role == role)
            .collect();
        let count = outbound_messages
            .iter()
            .map(|message| message.id + 1)
            .max()
            .unwrap_or(0);

        (0..count)
            .map(
                |id| match outbound_messages.iter().find(|message| message.id == id) {
                    Some(message) => {
                        let body = Self::body(message, &quote!('static));

                        (
                            quote!(<#body as #core::Message>::REQUIREMENTS),
                            quote!(<#body as #core::Message>::TIMEOUT),
                        )
                    }
                    None => (
                        quote!(&#core::MessageRequirementTable::NONE),
                        quote!(::std::option::Option::None),
                    ),
                },
            )
            .unzip()
    }
}
