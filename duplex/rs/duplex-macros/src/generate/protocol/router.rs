use duplex_protocol::{MessagePlan, Role, Symbol};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::ProtocolBuilder;
use crate::generate::names::NameProvider;

impl<'a> ProtocolBuilder<'a> {
    pub(super) fn generate_router(&self) -> TokenStream2 {
        let core = self.core;
        let local = self.plan.local();
        let peer = NameProvider::pascal(Symbol::Declared(&local.name));
        let router = NameProvider::pascal(Symbol::Router(local));
        let inbound_messages: Vec<&MessagePlan<'_>> = self.plan.inbound_messages().collect();
        let methods = inbound_messages.iter().map(|message| {
            let method = NameProvider::snake(Symbol::Handler(message.message));
            let body = Self::body(message, &quote!('static));

            if message.has_response() {
                let (_, responder) = self.exchange(message);

                quote! {
                    fn #method(
                        &self,
                        message: #body,
                        responder: #responder,
                        next: #core::NextMessage<#body>,
                    ) {
                        next.call(message, responder);
                    }
                }
            } else {
                let reference = self.notification_ref(message, &quote!('_));

                quote! {
                    fn #method(&self, message: #reference, next: #core::NextNotification<#body>) {
                        next.call(message);
                    }
                }
            }
        });
        let routes = inbound_messages.iter().map(|message| {
            let method = NameProvider::snake(Symbol::Handler(message.message));
            let body = Self::body(message, &quote!('static));

            if message.has_response() {
                quote! {
                    {
                        let router = ::std::rc::Rc::clone(&router);
                        channel.handle(move |message: #body, responder, next| {
                            router.#method(message, responder, next);
                        });
                    }
                }
            } else {
                quote! {
                    {
                        let router = ::std::rc::Rc::clone(&router);
                        channel.on::<#body>(move |message, next| router.#method(message, next));
                    }
                }
            }
        });
        let dispatch_routes = inbound_messages.iter().map(|message| {
            let body = Self::body(message, &quote!('static));

            if message.has_response() {
                quote! {
                    {
                        let dispatch = ::std::rc::Rc::clone(&dispatch);
                        channel.handle(move |message: #body, responder, next| {
                            ::std::rc::Rc::clone(&dispatch).message(message, responder, next);
                        });
                    }
                }
            } else {
                quote! {
                    {
                        let dispatch = ::std::rc::Rc::clone(&dispatch);
                        channel.on::<#body>(move |message, next| {
                            ::std::rc::Rc::clone(&dispatch).notification::<#body>(message, next);
                        });
                    }
                }
            }
        });
        let protocol = NameProvider::pascal(Symbol::Declared(&self.plan.protocol().name));
        let handles_open =
            self.plan.role == Role::Provider && self.plan.protocol().handshake.is_declared();
        let (open_method, open_route) = if handles_open {
            (
                Some(quote! {
                    fn on_connect(
                        &self,
                        open: <#protocol as #core::Protocol>::Open,
                        pending: #core::PendingConnection<#protocol>,
                    );
                }),
                Some(quote! {
                    {
                        let router = ::std::rc::Rc::clone(&router);
                        channel.on_connect(move |open, pending| router.on_connect(open, pending));
                    }
                }),
            )
        } else {
            (None, None)
        };
        let unused = inbound_messages.is_empty() && !handles_open;
        let channel = if unused {
            quote!(_channel)
        } else {
            quote!(channel)
        };
        let router_binding = if unused {
            quote!(_router)
        } else {
            quote!(router)
        };
        let (dispatch_channel, dispatch_binding) = if unused {
            (quote!(_channel), quote!(_dispatch))
        } else {
            (quote!(channel), quote!(dispatch))
        };
        let docs = format!(
            " Routes the messages the {} `{}` handles: override a message's method to handle it; `next` passes it on.",
            self.plan.role.keyword(),
            local.name
        );

        quote! {
            #[doc = #docs]
            pub trait #router: 'static {
                #open_method
                #(#methods)*

                fn attach<D: #core::Driver>(self, #channel: &#core::Channel<#peer, D>)
                where
                    Self: Sized,
                {
                    let #router_binding = ::std::rc::Rc::new(self);
                    #open_route
                    #(#routes)*
                }
            }

            /// Installs one generic dispatcher for every inbound message of this protocol role.
            pub fn attach_dispatch<D: #core::Driver, T: #core::Dispatch>(
                #dispatch_channel: &#core::Channel<#peer, D>,
                #dispatch_binding: ::std::rc::Rc<T>,
            ) {
                #(#dispatch_routes)*
            }
        }
    }

    /// Generates an enum of local-peer notifications and a method subscribing a shared delivery callback.
    pub(super) fn generate_notifications(&self) -> Option<TokenStream2> {
        let core = self.core;
        let local = self.plan.local();
        let notifications: Vec<&MessagePlan<'_>> = self
            .plan
            .inbound_messages()
            .filter(|message| Self::is_notification(message))
            .collect();

        if notifications.is_empty() {
            return None;
        }

        let peer = NameProvider::pascal(Symbol::Declared(&local.name));
        let ident = NameProvider::pascal(Symbol::Notification(local));
        let variants: Vec<_> = notifications
            .iter()
            .map(|message| NameProvider::pascal(Symbol::Declared(&message.message.name)))
            .collect();
        let bodies: Vec<_> = notifications
            .iter()
            .map(|message| Self::body(message, &quote!('static)))
            .collect();
        let names = notifications.iter().map(|message| &message.message.name);
        let members =
            notifications
                .iter()
                .zip(&variants)
                .zip(&bodies)
                .map(|((message, variant), body)| {
                    let docs = NameProvider::docs(&message.message.docs);

                    quote! { #docs #variant(#body) }
                });
        let attachments =
            notifications
                .iter()
                .zip(&variants)
                .zip(&bodies)
                .map(|((message, variant), body)| {

                    let message_value = if self.has_reference(message) {
                        quote!(message.into_owned())
                    } else {
                        quote!(message)
                    };

                    quote! {
                        {
                            let deliver = ::std::clone::Clone::clone(&deliver);
                            channel.on::<#body>(move |message, _| deliver(Self::#variant(#message_value)));
                        }
                    }
                });
        let docs = format!(
            " The notifications the {} `{}` handles, as one type.",
            self.plan.role.keyword(),
            local.name
        );

        Some(quote! {
            #[doc = #docs]
            #[derive(::std::fmt::Debug, ::std::clone::Clone, ::std::cmp::PartialEq)]
            pub enum #ident {
                #(#members,)*
            }

            impl #ident {
                /// Passes every notification received by `channel` to `deliver`.
                pub fn attach<D: #core::Driver>(
                    channel: &#core::Channel<#peer, D>,
                    deliver: impl ::std::ops::Fn(Self) + ::std::clone::Clone + 'static,
                ) {
                    #(#attachments)*
                }

                /// Declared message name.
                #[must_use]
                pub fn name(&self) -> &'static str {
                    match self {
                        #(Self::#variants(_) => #names,)*
                    }
                }

                /// Message tags.
                #[must_use]
                pub fn tags(&self) -> &'static [#core::Tag] {
                    match self {
                        #(Self::#variants(_) => <#bodies as #core::Metadata>::TAGS,)*
                    }
                }
            }

            impl #core::FieldAccess for #ident {
                fn fields(&self) -> &'static [#core::FieldTagContainer] {
                    match self {
                        #(Self::#variants(message) => #core::FieldAccess::fields(message),)*
                    }
                }

                fn field(&self, name: &str) -> ::std::option::Option<#core::ValueRef<'_>> {
                    match self {
                        #(Self::#variants(message) => #core::FieldAccess::field(message, name),)*
                    }
                }
            }
        })
    }
}
