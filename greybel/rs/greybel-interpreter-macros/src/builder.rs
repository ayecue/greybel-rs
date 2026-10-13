use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Attribute, Ident, ItemFn};

use crate::options::Options;
use crate::signature::{Access, Argument, Input, Signature};

const ATTR_DOC: &str = "doc";

pub struct Builder<'a> {
    function: &'a ItemFn,
    signature: Signature,
    label: Option<String>,
    method: bool,
    interpreter: TokenStream2,
}

impl<'a> Builder<'a> {
    pub fn new(
        function: &'a ItemFn,
        signature: Signature,
        label: Option<String>,
        method: bool,
        interpreter: TokenStream2,
    ) -> Self {
        Self {
            function,
            signature,
            label,
            method,
            interpreter,
        }
    }

    pub fn from_input(options: Options, function: &'a mut ItemFn) -> syn::Result<Self> {
        let interpreter = quote!(::greybel_interpreter);
        let label = options.label.map(|label| label.value());
        let is_async = function.sig.asyncness.is_some();
        let signature = Signature::from_inputs(
            &mut function.sig.inputs,
            &interpreter,
            is_async,
            options.method,
        )?;

        Ok(Self::new(
            function,
            signature,
            label,
            options.method,
            interpreter,
        ))
    }

    fn ident(&self) -> &Ident {
        &self.function.sig.ident
    }

    fn is_async(&self) -> bool {
        self.function.sig.asyncness.is_some()
    }

    fn variables(&self) -> Vec<Ident> {
        (0..self.signature.arguments.len())
            .map(|index| format_ident!("__argument{}", index))
            .collect()
    }

    fn generate_bindings(&self, variables: &[Ident]) -> Vec<TokenStream2> {
        let interpreter = &self.interpreter;
        let mut bindings: Vec<(bool, TokenStream2)> = self
            .signature
            .arguments
            .iter()
            .zip(variables)
            .enumerate()
            .map(|(index, (argument, variable))| {
                let Argument {
                    ty,
                    name: parameter,
                    access,
                    ..
                } = argument;
                let mutable = *access == Access::Mutable;
                let (expected_type, read) = if mutable {
                    let referent = Access::referent(ty);

                    (
                        quote!(<#referent as #interpreter::FromArgumentMut<'_>>::TYPE_DESCRIPTION),
                        quote! {
                            <#referent as #interpreter::FromArgumentMut<'_>>::from_argument_mut(
                                &__values[#index],
                                __vm.heap_mut(),
                            )
                        },
                    )
                } else {
                    (
                        quote!(<#ty as #interpreter::FromArgument<'_, '_>>::TYPE_DESCRIPTION),
                        quote! {
                            <#ty as #interpreter::FromArgument<'_, '_>>::from_argument(
                                &__values[#index],
                                __vm.heap(),
                            )
                        },
                    )
                };
                let error = quote! {
                    #interpreter::RuntimeError::invalid_argument(#parameter, #expected_type)
                };
                let fail = if self.is_async() {
                    quote! {
                        return ::std::boxed::Box::pin(::core::future::ready(
                            #interpreter::NativeOutput::new(
                                ::core::result::Result::<(), _>::Err(#error),
                            ),
                        ))
                    }
                } else {
                    quote!(return Err(#error))
                };
                let pattern = if mutable {
                    quote!(mut #variable)
                } else {
                    quote!(#variable)
                };
                let binding = quote! {
                    let ::core::option::Option::Some(#pattern) = #read else {
                        #fail;
                    };
                };

                (mutable, binding)
            })
            .collect();

        bindings.sort_by_key(|(mutable, _)| *mutable);

        bindings.into_iter().map(|(_, binding)| binding).collect()
    }

    fn generate_hosts(&self) -> Vec<TokenStream2> {
        let interpreter = &self.interpreter;

        self.signature
            .inputs
            .iter()
            .filter_map(|input| match input {
                Input::Host(ty, variable) => Some(quote! {
                    let #variable = <#ty as #interpreter::FromVm>::from_vm(__vm);
                }),
                _ => None,
            })
            .collect()
    }

    fn generate_call(&self, variables: &[Ident], bindings: &[TokenStream2]) -> TokenStream2 {
        let interpreter = &self.interpreter;
        let ident = self.ident();
        let inputs = self.signature.inputs.iter().map(|input| match input {
            Input::Vm => quote!(__vm),
            Input::Host(_, variable) => quote!(#variable),
            Input::Argument(index) => {
                let variable = &variables[*index];

                if self.signature.arguments[*index].access == Access::Mutable {
                    quote!(&mut *#variable)
                } else {
                    quote!(#variable)
                }
            }
        });
        let call = quote!(super::#ident(#(#inputs),*));

        if self.is_async() {
            quote! {
                #(#bindings)*
                let future: #interpreter::NativeFuture = ::std::boxed::Box::pin(async move {
                    #interpreter::NativeOutput::new(#call.await)
                });
                future
            }
        } else {
            quote! {
                let __result = {
                    #(#bindings)*
                    #call
                };
                #interpreter::IntoNativeResult::into_native_return(__result, __vm.heap_mut())
            }
        }
    }

    pub fn generate(&self) -> syn::Result<TokenStream2> {
        let interpreter = &self.interpreter;
        let function = self.function;
        let ident = self.ident();
        let vis = &function.vis;
        let docs = function
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident(ATTR_DOC));
        let docs: Vec<&Attribute> = docs.collect();
        let label = match &self.label {
            Some(label) => quote!(::core::option::Option::Some(#label)),
            None => quote!(::core::option::Option::None),
        };
        let method = self.method;
        let params = self
            .signature
            .arguments
            .iter()
            .filter_map(|argument| argument.param.as_ref());
        let constructor = if self.is_async() {
            quote!(from_async_fn)
        } else {
            quote!(from_fn)
        };
        let variables = self.variables();
        let hosts = self.generate_hosts();
        let bindings = self.generate_bindings(&variables);
        let call = self.generate_call(&variables, &bindings);

        Ok(quote! {
            #function

            #(#docs)*
            #[allow(non_snake_case)]
            #vis mod #ident {
                #[allow(unused_imports)]
                use super::*;

                #[allow(clippy::used_underscore_binding)]
                pub static NATIVE: #interpreter::Native = #interpreter::Native::#constructor(
                    #label,
                    &[#(#params),*],
                    #method,
                    |__vm, __args| {
                        let _ = &__vm;
                        let __values = #interpreter::ArgContainer::values(__args);
                        let _ = __values;
                        #(#hosts)*
                        #call
                    },
                );
            }
        })
    }
}
