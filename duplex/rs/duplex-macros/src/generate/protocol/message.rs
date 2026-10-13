use duplex_protocol::{Encoding, MessagePlan, Protocol, ResponseBodyPlan, StreamPlan, Symbol};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::ProtocolBuilder;
use crate::generate::declaration::DeclarationBuilder;
use crate::generate::names::NameProvider;
use crate::generate::rust_type::{Form, RustTypeMapper};

impl<'a> ProtocolBuilder<'a> {
    pub(super) fn exchange(&self, plan: &MessagePlan<'_>) -> (TokenStream2, TokenStream2) {
        let core = self.core;
        let value = |encoding: Option<&Encoding<'_>>| {
            encoding.map_or_else(|| quote!(()), |encoding| self.types.owned_type(encoding))
        };
        let failure = plan.failure().map_or_else(
            || quote!(::std::convert::Infallible),
            |failure| self.types.owned_type(failure),
        );
        let (exchange, responder) = match &plan.response {
            None => (quote!(#core::NoResponse), quote!(())),
            Some(response) => match &response.body {
                ResponseBodyPlan::Value(response) => {
                    let response = self.types.owned_type(response);

                    (
                        quote!(#core::ValueResponse<#response, #failure>),
                        quote!(#core::Responder<#response, #failure>),
                    )
                }
                ResponseBodyPlan::Stream(StreamPlan {
                    start, end, item, ..
                }) => {
                    let (start, end, item) = (
                        value(start.as_ref()),
                        value(end.as_ref()),
                        self.types.owned_type(item),
                    );

                    (
                        quote!(#core::StreamResponse<#start, #end, #item, #failure>),
                        quote!(#core::StreamResponder<#start, #end, #item, #failure>),
                    )
                }
            },
        };
        let Some(StreamPlan { end, item, .. }) = &plan.stream else {
            return (exchange, responder);
        };
        let (end, item) = (value(end.as_ref()), self.types.owned_type(item));

        (
            quote!(#core::StreamExchange<#end, #item, #exchange>),
            quote!((#core::IncomingStream<#end, #item>, #responder)),
        )
    }

    pub(super) fn generate_message(&self, plan: &'a MessagePlan<'a>) -> TokenStream2 {
        let core = self.core;
        let message = plan.message;
        let body = DeclarationBuilder::new(core, self.types).structure(
            &message.docs,
            &message.tags,
            &message.name,
            &plan.body,
            true,
        );
        let ty = Self::body(plan, &quote!('_));
        let owned_ty = Self::body(plan, &quote!('static));
        let id = plan.id;
        let since = plan.since;
        let sender = NameProvider::pascal(Symbol::Declared(&plan.sender.name));
        let (exchange, _) = self.exchange(plan);
        let timeout = message.timeout.map(|timeout| {
            let timeout = u64::from(timeout);

            quote! {
                const TIMEOUT: ::std::option::Option<::std::time::Duration> =
                    ::std::option::Option::Some(::std::time::Duration::from_millis(#timeout));
            }
        });
        let requirements = self.requirements(plan);
        let notification = self.notification(plan);

        quote! {
            #body

            #notification

            impl #core::Message for #ty {
                const ID: u32 = #id;
                const SINCE: u16 = #since;
                #timeout
                #requirements
                type Sender = #sender;
                type Exchange = #exchange;

                fn transcode(
                    reader: &mut #core::BinaryReader<'_>,
                    writer: &mut #core::BinaryWriter,
                ) -> ::std::result::Result<(), #core::Error> {
                    let message = <#owned_ty as #core::BinaryDecode>::read(reader)?;
                    reader.finish()?;
                    #core::BinaryEncode::write(&message, writer)
                }
            }
        }
    }

    pub(super) fn is_notification(plan: &MessagePlan<'_>) -> bool {
        plan.response.is_none() && plan.stream.is_none()
    }

    /// Returns whether notification handlers take `plan` as borrowed `Ref` rather than an owned message.
    /// Strings, bytes, and records borrow; lists of plain values do not.
    pub(super) fn has_reference(&self, plan: &MessagePlan<'_>) -> bool {
        !self.types.fields_are(&plan.body, RustTypeMapper::is_plain)
    }

    /// Returns the notification handler argument type for `plan`, applying `lifetime` to borrowed data.
    pub(super) fn notification_ref(
        &self,
        plan: &MessagePlan<'_>,
        lifetime: &TokenStream2,
    ) -> TokenStream2 {
        if self.has_reference(plan) {
            let ident = NameProvider::reference(&plan.message.name);

            quote!(#ident<#lifetime>)
        } else {
            Self::body(plan, &quote!('static))
        }
    }

    fn notification(&self, plan: &'a MessagePlan<'a>) -> Option<TokenStream2> {
        if !Self::is_notification(plan) {
            return None;
        }

        let core = self.core;
        let builder = DeclarationBuilder::new(core, self.types);
        let owned_ty = Self::body(plan, &quote!('static));
        let reference = if self.has_reference(plan) {
            let form = if RustTypeMapper::borrows(&plan.body) {
                Form::Reference
            } else {
                Form::Value
            };

            builder.reference(&plan.message.name, &plan.body, &owned_ty, form)
        } else {
            builder.borrow_decode(&owned_ty)
        };
        let ty = Self::body(plan, &quote!('_));
        let notification = self.notification_ref(plan, &quote!('a));

        Some(quote! {
            #reference

            impl #core::NotificationMessage for #ty {
                type Ref<'a> = #notification;
            }
        })
    }

    fn requirements(&self, plan: &MessagePlan<'_>) -> Option<TokenStream2> {
        let core = self.core;
        let any = quote!(#core::Validation::Any);
        let validation = |encoding: Option<&Encoding<'_>>| {
            encoding
                .filter(|encoding| self.types.requires(encoding))
                .map(|encoding| self.types.validation(encoding))
        };
        let items = |stream: Option<&StreamPlan<'_>>| {
            stream
                .filter(|stream| !stream.items.is_any())
                .map(|stream| RustTypeMapper::items(core, stream.items, &any))
        };
        let response = plan.response.as_ref();
        let response_stream = response.and_then(|response| match &response.body {
            ResponseBodyPlan::Stream(stream) => Some(stream),
            ResponseBodyPlan::Value(_) => None,
        });
        let response_end = response.and_then(|response| match &response.body {
            ResponseBodyPlan::Stream(stream) => stream.end.as_ref(),
            ResponseBodyPlan::Value(value) => Some(value),
        });
        let validations = [
            ("message_items", items(plan.stream.as_ref())),
            ("response_items", items(response_stream)),
            (
                "message_item",
                validation(plan.stream.as_ref().map(|stream| &stream.item)),
            ),
            (
                "message_end",
                validation(plan.stream.as_ref().and_then(|stream| stream.end.as_ref())),
            ),
            (
                "response_start",
                validation(response_stream.and_then(|stream| stream.start.as_ref())),
            ),
            (
                "response_item",
                validation(response_stream.map(|stream| &stream.item)),
            ),
            ("response_end", validation(response_end)),
            ("failure", validation(plan.failure())),
        ];
        let limit = plan.limit;
        let size = |stream: Option<&StreamPlan<'_>>| stream.and_then(|stream| stream.item_size);
        let (message_item_size, response_item_size) =
            (size(plan.stream.as_ref()), size(response_stream));

        if limit == Protocol::DEFAULT_LIMIT
            && message_item_size.is_none()
            && response_item_size.is_none()
            && validations
                .iter()
                .all(|(_, validation)| validation.is_none())
        {
            return None;
        }

        let size = |size: Option<u32>| match size {
            Some(size) => quote!(Some(#size)),
            None => quote!(None),
        };
        let (message_item_size, response_item_size) =
            (size(message_item_size), size(response_item_size));
        let validations = validations.into_iter().map(|(name, validation)| {
            let name = quote::format_ident!("{}", name);
            let validation = validation.unwrap_or_else(|| any.clone());

            quote!(#name: #validation)
        });

        Some(quote! {
            const REQUIREMENTS: &'static #core::MessageRequirementTable = &#core::MessageRequirementTable {
                limit: #limit,
                message_item_size: #message_item_size,
                response_item_size: #response_item_size,
                #(#validations,)*
            };
        })
    }

    pub(super) fn body(plan: &MessagePlan<'_>, lifetime: &TokenStream2) -> TokenStream2 {
        let ident = NameProvider::pascal(Symbol::Declared(&plan.message.name));

        if RustTypeMapper::borrows(&plan.body) {
            quote!(#ident<#lifetime>)
        } else {
            quote!(#ident)
        }
    }
}
