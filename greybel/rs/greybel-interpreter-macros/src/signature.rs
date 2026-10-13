use greybel_symbols::names::SELF;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    Expr, ExprLit, ExprUnary, FnArg, GenericArgument, Ident, Lit, Pat, PatType, PathArguments,
    Type, TypePath, UnOp,
};

const ATTR_DEFAULT: &str = "default";
const ATTR_NAME: &str = "name";
const ATTR_HOST: &str = "host";
const VM_TYPE: &str = "Vm";
const OPTION_TYPE: &str = "Option";
const MAP_CONTENTS_TYPE: &str = "ValueMap";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Value,
    Contents,
    Mutable,
}

impl Access {
    pub fn referent(ty: &Type) -> &Type {
        match ty {
            Type::Reference(reference) => &reference.elem,
            other => other,
        }
    }

    fn of(ty: &Type) -> Self {
        match ty {
            Type::Reference(reference) if reference.mutability.is_some() => Self::Mutable,
            Type::Reference(reference) => match &*reference.elem {
                Type::Slice(_) => Self::Contents,
                Type::Path(path) if Parameter::is_named(path, MAP_CONTENTS_TYPE) => Self::Contents,
                _ => Self::Value,
            },
            Type::Path(path) if Parameter::is_named(path, OPTION_TYPE) => {
                let inner =
                    path.path
                        .segments
                        .last()
                        .and_then(|segment| match &segment.arguments {
                            PathArguments::AngleBracketed(arguments) => {
                                arguments.args.iter().find_map(|argument| match argument {
                                    GenericArgument::Type(ty) => Some(Self::of(ty)),
                                    _ => None,
                                })
                            }
                            _ => None,
                        });

                inner.unwrap_or(Self::Value)
            }
            _ => Self::Value,
        }
    }
}

pub struct Argument {
    pub ty: Box<Type>,
    pub name: String,
    pub param: Option<TokenStream2>,
    pub access: Access,
}

pub enum Input {
    Vm,
    Host(Box<Type>, Ident),
    Argument(usize),
}

pub struct Signature {
    pub inputs: Vec<Input>,
    pub arguments: Vec<Argument>,
}

impl Signature {
    pub fn new(inputs: Vec<Input>, arguments: Vec<Argument>) -> Self {
        Self { inputs, arguments }
    }

    pub fn from_inputs(
        inputs: &mut Punctuated<FnArg, Comma>,
        interpreter: &TokenStream2,
        is_async: bool,
        method: bool,
    ) -> syn::Result<Self> {
        let mut order = Vec::new();
        let mut arguments = Vec::new();

        for (index, input) in inputs.iter_mut().enumerate() {
            let FnArg::Typed(input) = input else {
                return Err(syn::Error::new_spanned(input, "natives take no `self`"));
            };
            let mut parameter = Parameter(input);

            if index == 0 && parameter.is_vm() {
                if is_async {
                    return Err(syn::Error::new_spanned(
                        parameter.0,
                        "async natives can't take the VM: their future runs after the call",
                    ));
                }

                order.push(Input::Vm);
                continue;
            }

            if parameter.take_host() {
                let variable = format_ident!("__host{}", order.len());

                order.push(Input::Host(parameter.0.ty.clone(), variable));
                continue;
            }

            let (name, param) = if method && arguments.is_empty() {
                (SELF.to_owned(), parameter.receiver_param(interpreter)?)
            } else {
                let (name, param) = parameter.param(interpreter)?;

                (name, Some(param))
            };

            order.push(Input::Argument(arguments.len()));

            let access = Access::of(&parameter.0.ty);

            parameter.check_access(access, is_async, &order, &arguments)?;
            arguments.push(Argument {
                ty: parameter.0.ty.clone(),
                name,
                param,
                access,
            });
        }

        if method && arguments.is_empty() {
            return Err(syn::Error::new_spanned(
                inputs,
                "a method native takes its receiver after the VM",
            ));
        }

        Ok(Self::new(order, arguments))
    }
}

struct Parameter<'a>(&'a mut PatType);

impl Parameter<'_> {
    fn check_access(
        &self,
        access: Access,
        is_async: bool,
        order: &[Input],
        arguments: &[Argument],
    ) -> syn::Result<()> {
        let error = |message: &str| Err(syn::Error::new_spanned(&self.0.ty, message));

        if access == Access::Value {
            return Ok(());
        }

        if is_async {
            return error(
                "async natives can't take list or map contents: their future outlives the call; \
                 take a `List` / `Map` or an owned `Vec<Value>`",
            );
        }

        if order.iter().any(|input| matches!(input, Input::Vm)) {
            return error(
                "a native taking `vm: &mut Vm` can't also take list or map contents; \
                 take a `List` / `Map` and read it through the VM",
            );
        }

        let mutable = arguments
            .iter()
            .any(|argument| argument.access == Access::Mutable);
        let borrowed = arguments
            .iter()
            .any(|argument| argument.access != Access::Value);

        if mutable || (access == Access::Mutable && borrowed) {
            return error(
                "a native taking `&mut` contents can't take any other list or map contents: \
                 both could be the same list",
            );
        }

        Ok(())
    }

    fn is_named(path: &TypePath, name: &str) -> bool {
        path.path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name)
    }

    fn is_vm(&self) -> bool {
        let Type::Reference(reference) = &*self.0.ty else {
            return false;
        };
        let Type::Path(path) = &*reference.elem else {
            return false;
        };

        reference.mutability.is_some() && Self::is_named(path, VM_TYPE)
    }

    fn take_host(&mut self) -> bool {
        let attrs = &mut self.0.attrs;
        let before = attrs.len();

        attrs.retain(|attr| !attr.path().is_ident(ATTR_HOST));

        attrs.len() != before
    }

    fn strip(&mut self) {
        self.0
            .attrs
            .retain(|attr| !attr.path().is_ident(ATTR_DEFAULT) && !attr.path().is_ident(ATTR_NAME));
    }

    fn receiver_param(&mut self, interpreter: &TokenStream2) -> syn::Result<Option<TokenStream2>> {
        let default = self
            .0
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident(ATTR_DEFAULT))
            .map(|attr| Self::default_value(&attr.parse_args::<Expr>()?, interpreter))
            .transpose()?;

        self.strip();

        Ok(default.map(|default| quote!(#interpreter::NativeParam::with_default(#SELF, #default))))
    }

    fn param(&mut self, interpreter: &TokenStream2) -> syn::Result<(String, TokenStream2)> {
        let mut name = match &*self.0.pat {
            Pat::Ident(pat) => pat.ident.unraw().to_string(),
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "native parameters must be plain names",
                ));
            }
        };
        let mut default = None;

        for attr in &self.0.attrs {
            if attr.path().is_ident(ATTR_NAME) {
                match &attr.meta.require_name_value()?.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(text),
                        ..
                    }) => name = text.value(),
                    other => return Err(syn::Error::new_spanned(other, "expected a string")),
                }
            } else if attr.path().is_ident(ATTR_DEFAULT) {
                default = Some(Self::default_value(
                    &attr.parse_args::<Expr>()?,
                    interpreter,
                )?);
            }
        }

        self.strip();

        let param = match default {
            Some(default) => quote!(#interpreter::NativeParam::with_default(#name, #default)),
            None => quote!(#interpreter::NativeParam::new(#name)),
        };

        Ok((name, param))
    }

    fn default_value(expr: &Expr, interpreter: &TokenStream2) -> syn::Result<TokenStream2> {
        let number = |lit: &Lit, negate: bool| -> syn::Result<TokenStream2> {
            let value = match lit {
                Lit::Int(int) => int.base10_parse::<f64>()?,
                Lit::Float(float) => float.base10_parse::<f64>()?,
                other => return Err(syn::Error::new_spanned(other, "expected a number")),
            };
            let value = if negate { -value } else { value };

            Ok(quote!(#interpreter::NativeDefault::Number(#value)))
        };

        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(text),
                ..
            }) => Ok(quote!(#interpreter::NativeDefault::String(#text))),
            Expr::Lit(ExprLit {
                lit: Lit::Bool(boolean),
                ..
            }) => {
                let value = if boolean.value { 1.0f64 } else { 0.0f64 };

                Ok(quote!(#interpreter::NativeDefault::Number(#value)))
            }
            Expr::Lit(ExprLit { lit, .. }) => number(lit, false),
            Expr::Unary(ExprUnary {
                op: UnOp::Neg(_),
                expr,
                ..
            }) => match &**expr {
                Expr::Lit(ExprLit { lit, .. }) => number(lit, true),
                other => Err(syn::Error::new_spanned(other, "expected a number")),
            },
            other => Err(syn::Error::new_spanned(
                other,
                "a default must be a number, string or boolean literal",
            )),
        }
    }
}
