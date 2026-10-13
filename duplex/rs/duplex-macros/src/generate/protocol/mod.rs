mod channel;
mod declaration;
mod message;
mod peer;
mod router;

use duplex_protocol::{Role, RolePlan};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::rust_type::RustTypeMapper;

pub struct ProtocolBuilder<'a> {
    core: &'a TokenStream2,
    plan: RolePlan<'a, 'a>,
    types: &'a RustTypeMapper<'a>,
}

impl<'a> ProtocolBuilder<'a> {
    pub fn new(
        core: &'a TokenStream2,
        plan: RolePlan<'a, 'a>,
        types: &'a RustTypeMapper<'a>,
    ) -> Self {
        Self { core, plan, types }
    }

    pub fn generate(&self) -> TokenStream2 {
        let protocol = self.generate_protocol();
        let peers = Role::ALL.map(|role| self.generate_peer(role));
        let messages = self
            .plan
            .plan
            .messages
            .iter()
            .map(|message| self.generate_message(message));
        let router = self.generate_router();
        let channels = Role::ALL.map(|role| self.generate_channel(role));
        let notifications = self.generate_notifications();

        quote! {
            #protocol
            #(#peers)*
            #(#messages)*
            #router
            #(#channels)*
            #notifications
        }
    }
}
