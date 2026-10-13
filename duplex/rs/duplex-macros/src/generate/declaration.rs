use duplex_protocol::{
    ConstantPlan, DocumentationRef, FieldsPlan, MemberPlan, PayloadPlan, Symbol, TagList,
    TypeContent, TypePlan, VariantPlan,
};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use super::codec::FieldsCodec;
use super::names::NameProvider;
use super::rust_type::{Form, RustTypeMapper};
use super::tags::TagsBuilder;

pub struct DeclarationBuilder<'a> {
    core: &'a TokenStream2,
    types: &'a RustTypeMapper<'a>,
}

impl<'a> DeclarationBuilder<'a> {
    pub fn new(core: &'a TokenStream2, types: &'a RustTypeMapper<'a>) -> Self {
        Self { core, types }
    }

    pub fn declaration(&self, plan: &'a TypePlan<'a>) -> TokenStream2 {
        let name = plan.name();
        let docs = &plan.declaration.docs;
        let tags = &plan.declaration.tags;
        let declaration = match &plan.content {
            TypeContent::Struct(fields) => self.structure(docs, tags, name, fields, false),
            TypeContent::Union(variants) => self.union(docs, tags, name, &plan.content, variants),
            TypeContent::Enumeration(members) => self.enumeration(docs, tags, name, members),
        };
        let ident = NameProvider::pascal(Symbol::Declared(name));
        let reference = if self
            .types
            .content_is(&plan.content, RustTypeMapper::is_plain)
        {
            self.borrow_decode(&quote!(#ident))
        } else {
            match &plan.content {
                TypeContent::Struct(fields) => {
                    self.reference(name, fields, &quote!(#ident), Form::Value)
                }
                TypeContent::Union(variants) => self.reference_union(name, &plan.content, variants),
                TypeContent::Enumeration(_) => self.borrow_decode(&quote!(#ident)),
            }
        };

        quote! {
            #declaration
            #reference
        }
    }

    /// Generates `BorrowDecode` for `ty`, delegating to owned decoding if no input is borrowed.
    pub fn borrow_decode(&self, ty: &TokenStream2) -> TokenStream2 {
        let core = self.core;

        quote! {
            impl<'de> #core::BorrowDecode<'de> for #ty {
                fn read_ref(reader: &mut #core::BinaryReader<'de>) -> ::std::result::Result<Self, #core::Error> {
                    <Self as #core::BinaryDecode>::read(reader)
                }

                fn read_ref_with_validation(
                    reader: &mut #core::BinaryReader<'de>,
                    validation: &#core::Validation,
                ) -> ::std::result::Result<Self, #core::Error> {
                    <Self as #core::ValidatedDecode>::read_with_validation(reader, validation)
                }
            }
        }
    }

    /// Generates the `Ref` form for struct or message `name` with owned representation `owned_ty` and conversion form
    /// `form`.
    pub fn reference(
        &self,
        name: &str,
        fields: &'a FieldsPlan<'a>,
        owned_ty: &TokenStream2,
        form: Form,
    ) -> TokenStream2 {
        let core = self.core;
        let ident = NameProvider::reference(name);
        let owned_ident = NameProvider::pascal(Symbol::Declared(name));
        let docs = format!(" A `{owned_ident}` borrowing the bytes it was read from.");
        let derives = Self::derives(false, self.types.fields_are(fields, RustTypeMapper::is_eq));
        let members = fields.fields().map(|field| {
            let docs = NameProvider::docs(&field.field.docs);
            let ident = NameProvider::field(field.field);
            let ty = self.types.field(field, Form::Ref);

            quote! { #docs pub #ident: #ty }
        });
        let codec = FieldsCodec::new(core, self.types, fields, Form::Ref);
        let read = codec.read();
        let construct = codec.construct();
        let into_owned = codec.owned_fields(form, |field| {
            let ident = NameProvider::field(field);

            quote!(self.#ident)
        });
        let decode = Self::reference_decode(
            core,
            &ident,
            &quote! {
                #read
                ::std::result::Result::Ok(Self { #construct })
            },
        );

        quote! {
            #[doc = #docs]
            #derives
            pub struct #ident<'a> {
                #(#members,)*
            }

            #decode

            impl<'a> #ident<'a> {
                /// Copies borrowed data into the decoded owned value.
                #[must_use]
                pub fn into_owned(self) -> #owned_ty {
                    #owned_ident { #into_owned }
                }
            }
        }
    }

    fn reference_union(
        &self,
        name: &str,
        content: &TypeContent<'_>,
        variants: &'a [VariantPlan<'a>],
    ) -> TokenStream2 {
        let core = self.core;
        let ident = NameProvider::reference(name);
        let owned_name = NameProvider::pascal(Symbol::Declared(name));
        let docs = format!(" A `{owned_name}` borrowing the bytes it was read from.");
        let derives = Self::derives(false, self.types.content_is(content, RustTypeMapper::is_eq));
        let members = variants.iter().map(|plan| {
            let docs = NameProvider::docs(&plan.variant.docs);
            let ident = NameProvider::pascal(Symbol::Declared(&plan.variant.name));

            match &plan.payload {
                PayloadPlan::Unit => quote! { #docs #ident },
                PayloadPlan::Value(ty) => {
                    let ty = self.types.reference(ty);

                    quote! { #docs #ident(#ty) }
                }
                PayloadPlan::Fields(fields) => {
                    let fields = fields.fields().map(|field| {
                        let docs = NameProvider::docs(&field.field.docs);
                        let ident = NameProvider::field(field.field);
                        let ty = self.types.field(field, Form::Ref);

                        quote! { #docs #ident: #ty }
                    });

                    quote! { #docs #ident { #(#fields),* } }
                }
            }
        });
        let reads = variants.iter().map(|plan| {
            let index = plan.index;
            let variant = NameProvider::pascal(Symbol::Declared(&plan.variant.name));

            match &plan.payload {
                PayloadPlan::Unit => quote! { #index => Self::#variant, },
                PayloadPlan::Value(_) => quote! {
                    #index => Self::#variant(#core::BorrowDecode::read_ref(reader)?),
                },
                PayloadPlan::Fields(fields) => {
                    let codec = FieldsCodec::new(core, self.types, fields, Form::Ref);
                    let read = codec.read();
                    let construct = codec.construct();

                    quote! {
                        #index => {
                            #read
                            Self::#variant { #construct }
                        }
                    }
                }
            }
        });
        let owns = variants.iter().map(|plan| {
            let variant = NameProvider::pascal(Symbol::Declared(&plan.variant.name));

            match &plan.payload {
                PayloadPlan::Unit => quote! { Self::#variant => #owned_name::#variant, },
                PayloadPlan::Value(ty) => {
                    let value = self.types.owned_value(&quote!(value), ty, Form::Value);

                    quote! { Self::#variant(value) => #owned_name::#variant(#value), }
                }
                PayloadPlan::Fields(fields) => {
                    let codec = FieldsCodec::new(core, self.types, fields, Form::Ref);
                    let bindings = Self::bindings(fields);
                    let into_owned = codec.owned_fields(Form::Value, |field| {
                        let binding = FieldsCodec::binding(field);

                        quote!(#binding)
                    });

                    quote! { Self::#variant { #bindings } => #owned_name::#variant { #into_owned }, }
                }
            }
        });
        let decode = Self::reference_decode(
            core,
            &ident,
            &quote! {
                ::std::result::Result::Ok(match reader.u32()? {
                    #(#reads)*
                    index => {
                        return ::std::result::Result::Err(#core::Error::Decode(#core::DecodeFailure::UnknownVariant { index, name: #name }));
                    }
                })
            },
        );
        let owned_ty = self.types.declared_type(name);

        quote! {
            #[doc = #docs]
            #derives
            pub enum #ident<'a> {
                #(#members,)*
            }

            #decode

            impl<'a> #ident<'a> {
                /// Copies borrowed data into the decoded owned value.
                #[must_use]
                pub fn into_owned(self) -> #owned_ty {
                    match self {
                        #(#owns)*
                    }
                }
            }
        }
    }

    fn reference_decode(core: &TokenStream2, ident: &Ident, read: &TokenStream2) -> TokenStream2 {
        quote! {
            impl<'a> #core::BorrowDecode<'a> for #ident<'a> {
                fn read_ref(reader: &mut #core::BinaryReader<'a>) -> ::std::result::Result<Self, #core::Error> {
                    #read
                }

                fn read_ref_with_validation(
                    reader: &mut #core::BinaryReader<'a>,
                    _: &#core::Validation,
                ) -> ::std::result::Result<Self, #core::Error> {
                    Self::read_ref(reader)
                }
            }
        }
    }

    pub fn constant(&self, plan: &ConstantPlan<'_>) -> TokenStream2 {
        let docs = NameProvider::docs(&plan.constant.docs);
        let ident = NameProvider::ident(&plan.constant.name);
        let ty = self.types.constant(&plan.ty);
        let value = self.types.constant_value(&plan.value, &plan.ty);

        quote! {
            #docs
            pub const #ident: #ty = #value;
        }
    }

    pub fn structure(
        &self,
        docs: &DocumentationRef,
        tags: &TagList,
        name: &str,
        fields: &'a FieldsPlan<'a>,
        borrowed: bool,
    ) -> TokenStream2 {
        let core = self.core;
        let docs = NameProvider::docs(docs);
        let ident = NameProvider::pascal(Symbol::Declared(name));
        let lifetime = (borrowed && RustTypeMapper::borrows(fields)).then(|| quote!(<'a>));
        let form = if lifetime.is_some() {
            Form::Reference
        } else {
            Form::Value
        };
        let derives = Self::derives(
            self.types.fields_are(fields, RustTypeMapper::is_copy) && lifetime.is_none(),
            self.types.fields_are(fields, RustTypeMapper::is_eq),
        );
        let members = fields.fields().map(|field| {
            let docs = NameProvider::docs(&field.field.docs);
            let ident = NameProvider::field(field.field);
            let ty = self.types.field(field, form);

            quote! { #docs pub #ident: #ty }
        });
        let codec = FieldsCodec::new(core, self.types, fields, form);
        let write = codec.write(|field| {
            let ident = NameProvider::field(field);

            quote!(&self.#ident)
        });
        let read = codec.read();
        let construct = codec.construct();
        let validation = codec.validate(|field| {
            let ident = NameProvider::field(field);

            quote!(&self.#ident)
        });
        let impl_generics = lifetime.clone();
        let metadata = self.tags().structure(
            &quote!(#ident #lifetime),
            impl_generics.as_ref(),
            tags,
            fields,
        );
        let default = self.default(&ident, lifetime.as_ref(), fields);
        let validation_impl = Self::emit_validation(
            core,
            &quote!(#ident #lifetime),
            impl_generics.as_ref(),
            &validation,
        );

        quote! {
            #docs
            #derives
            pub struct #ident #lifetime {
                #(#members,)*
            }

            impl #impl_generics #core::BinaryEncode for #ident #lifetime {
                fn write(&self, writer: &mut #core::BinaryWriter) -> ::std::result::Result<(), #core::Error> {
                    #write
                    ::std::result::Result::Ok(())
                }
            }

            impl #impl_generics #core::BinaryDecode for #ident #lifetime {
                fn read(reader: &mut #core::BinaryReader<'_>) -> ::std::result::Result<Self, #core::Error> {
                    #read
                    ::std::result::Result::Ok(Self { #construct })
                }
            }

            #validation_impl

            #default

            #metadata
        }
    }

    fn tags(&self) -> TagsBuilder<'a> {
        TagsBuilder::new(self.core, self.types.definition())
    }

    fn emit_validation(
        core: &TokenStream2,
        ty: &TokenStream2,
        generics: Option<&TokenStream2>,
        validation: &TokenStream2,
    ) -> TokenStream2 {
        quote! {
            impl #generics #core::Validatable for #ty {
                fn validate(&self, _: &#core::Validation) -> ::std::result::Result<(), #core::Error> {
                    #validation
                    ::std::result::Result::Ok(())
                }
            }

            impl #generics #core::ValidatedDecode for #ty {
                fn read_with_validation(
                    reader: &mut #core::BinaryReader<'_>,
                    _: &#core::Validation,
                ) -> ::std::result::Result<Self, #core::Error> {
                    #core::BinaryDecode::read(reader)
                }
            }
        }
    }

    fn default(
        &self,
        ident: &Ident,
        lifetime: Option<&TokenStream2>,
        fields: &FieldsPlan<'_>,
    ) -> Option<TokenStream2> {
        let values: Vec<TokenStream2> = fields
            .fields()
            .map(|field| {
                let value = match (&field.default, field.field.optional) {
                    (Some(default), _) => self.types.value(default, &field.ty),
                    (None, true) => quote!(::std::option::Option::None),
                    (None, false) => return None,
                };
                let name = NameProvider::field(field.field);

                Some(quote!(#name: #value))
            })
            .collect::<Option<_>>()?;

        if values.is_empty() {
            return None;
        }

        Some(quote! {
            impl #lifetime ::std::default::Default for #ident #lifetime {
                fn default() -> Self {
                    Self { #(#values),* }
                }
            }
        })
    }

    fn union(
        &self,
        docs: &DocumentationRef,
        tags: &TagList,
        name: &str,
        content: &TypeContent<'_>,
        variants: &'a [VariantPlan<'a>],
    ) -> TokenStream2 {
        let core = self.core;
        let docs = NameProvider::docs(docs);
        let ident = NameProvider::pascal(Symbol::Declared(name));
        let derives = Self::derives(
            self.types.content_is(content, RustTypeMapper::is_copy),
            self.types.content_is(content, RustTypeMapper::is_eq),
        );
        let members = variants.iter().map(|plan| {
            let docs = NameProvider::docs(&plan.variant.docs);
            let ident = NameProvider::pascal(Symbol::Declared(&plan.variant.name));

            match &plan.payload {
                PayloadPlan::Unit => quote! { #docs #ident },
                PayloadPlan::Value(ty) => {
                    let ty = self.types.owned_type(ty);

                    quote! { #docs #ident(#ty) }
                }
                PayloadPlan::Fields(fields) => {
                    let fields = fields.fields().map(|field| {
                        let docs = NameProvider::docs(&field.field.docs);
                        let ident = NameProvider::field(field.field);
                        let ty = self.types.field(field, Form::Value);

                        quote! { #docs #ident: #ty }
                    });

                    quote! { #docs #ident { #(#fields),* } }
                }
            }
        });
        let writes = variants.iter().map(|plan| {
            let index = plan.index;
            let variant = NameProvider::pascal(Symbol::Declared(&plan.variant.name));
            let guard = Self::version_guard(core, plan.since);

            match &plan.payload {
                PayloadPlan::Unit => quote! {
                    Self::#variant => {
                        #guard
                        writer.u32(#index);
                    }
                },
                PayloadPlan::Value(_) => quote! {
                    Self::#variant(value) => {
                        #guard
                        writer.u32(#index);
                        #core::BinaryEncode::write(value, writer)?;
                    }
                },
                PayloadPlan::Fields(fields) => {
                    let codec = FieldsCodec::new(core, self.types, fields, Form::Value);
                    let bindings = Self::bindings(fields);
                    let write = codec.write(|field| {
                        let binding = FieldsCodec::binding(field);

                        quote!(#binding)
                    });

                    quote! {
                        Self::#variant { #bindings } => {
                            #guard
                            writer.u32(#index);
                            #write
                        }
                    }
                }
            }
        });
        let reads = variants.iter().map(|plan| {
            let index = plan.index;
            let variant = NameProvider::pascal(Symbol::Declared(&plan.variant.name));

            match &plan.payload {
                PayloadPlan::Unit => quote! { #index => Self::#variant, },
                PayloadPlan::Value(_) => quote! {
                    #index => Self::#variant(#core::BinaryDecode::read(reader)?),
                },
                PayloadPlan::Fields(fields) => {
                    let codec = FieldsCodec::new(core, self.types, fields, Form::Value);
                    let read = codec.read();
                    let construct = codec.construct();

                    quote! {
                        #index => {
                            #read
                            Self::#variant { #construct }
                        }
                    }
                }
            }
        });
        let validations = variants.iter().map(|plan| {
            let variant = NameProvider::pascal(Symbol::Declared(&plan.variant.name));

            match &plan.payload {
                PayloadPlan::Unit => quote! { Self::#variant => {} },
                PayloadPlan::Value(ty) => match self.types.verify(&quote!(value), ty, None) {
                    Some(validation) => quote! { Self::#variant(value) => { #validation } },
                    None => quote! { Self::#variant(..) => {} },
                },
                PayloadPlan::Fields(fields) => {
                    let codec = FieldsCodec::new(core, self.types, fields, Form::Value);
                    let validation = codec.validate(|field| {
                        let binding = FieldsCodec::binding(field);

                        quote!(#binding)
                    });

                    if validation.is_empty() {
                        return quote! { Self::#variant { .. } => {} };
                    }

                    let bindings = Self::bindings(fields);

                    quote! {
                        #[allow(unused_variables)]
                        Self::#variant { #bindings } => { #validation }
                    }
                }
            }
        });
        let emit_validation = Self::emit_validation(
            core,
            &quote!(#ident),
            None,
            &quote! {
                match self {
                    #(#validations)*
                }
            },
        );
        let metadata = self.union_tags(&ident, tags, variants);

        quote! {
            #docs
            #derives
            pub enum #ident {
                #(#members,)*
            }

            impl #core::BinaryEncode for #ident {
                fn write(&self, writer: &mut #core::BinaryWriter) -> ::std::result::Result<(), #core::Error> {
                    match self {
                        #(#writes)*
                    }
                    ::std::result::Result::Ok(())
                }
            }

            impl #core::BinaryDecode for #ident {
                fn read(reader: &mut #core::BinaryReader<'_>) -> ::std::result::Result<Self, #core::Error> {
                    ::std::result::Result::Ok(match reader.u32()? {
                        #(#reads)*
                        index => {
                            return ::std::result::Result::Err(#core::Error::Decode(#core::DecodeFailure::UnknownVariant { index, name: #name }));
                        }
                    })
                }
            }

            #emit_validation

            #metadata
        }
    }

    /// Generates union `Metadata` and `AsValueRef` implementations.
    fn union_tags(
        &self,
        ident: &Ident,
        tags: &TagList,
        variants: &[VariantPlan<'_>],
    ) -> TokenStream2 {
        let core = self.core;
        let builder = self.tags();
        let union_tags = builder.tags(tags);
        let values = variants.iter().map(|plan| {
            let variant = NameProvider::pascal(Symbol::Declared(&plan.variant.name));
            let name = &plan.variant.name;
            let tags = builder.tags(&plan.variant.tags);

            match &plan.payload {
                PayloadPlan::Unit => quote! {
                    Self::#variant => #core::ValueRef::Variant {
                        name: #name,
                        tags: #tags,
                        fields: &[],
                        values: ::std::vec::Vec::new(),
                    },
                },
                PayloadPlan::Value(_) => quote! {
                    Self::#variant(value) => #core::ValueRef::Variant {
                        name: #name,
                        tags: #tags,
                        fields: &[],
                        values: ::std::vec![#core::AsValueRef::as_value_ref(value)],
                    },
                },
                PayloadPlan::Fields(fields) => {
                    let bindings = Self::bindings(fields);
                    let field_tags = builder.field_tags(fields);
                    let values = fields.fields().map(|field| {
                        let binding = FieldsCodec::binding(field.field);

                        quote!(#core::AsValueRef::as_value_ref(#binding))
                    });

                    quote! {
                        Self::#variant { #bindings } => #core::ValueRef::Variant {
                            name: #name,
                            tags: #tags,
                            fields: #field_tags,
                            values: ::std::vec![#(#values),*],
                        },
                    }
                }
            }
        });
        let scrutinee = if variants.is_empty() {
            quote!(*self)
        } else {
            quote!(self)
        };

        quote! {
            impl #core::Metadata for #ident {
                const TAGS: &'static [#core::Tag] = #union_tags;
            }

            impl #core::AsValueRef for #ident {
                fn as_value_ref(&self) -> #core::ValueRef<'_> {
                    match #scrutinee {
                        #(#values)*
                    }
                }
            }
        }
    }

    fn enumeration(
        &self,
        docs: &DocumentationRef,
        tags: &TagList,
        name: &str,
        members: &[MemberPlan<'_>],
    ) -> TokenStream2 {
        let core = self.core;
        let docs = NameProvider::docs(docs);
        let ident = NameProvider::pascal(Symbol::Declared(name));
        let derives = Self::derives(true, true);
        let variants = members.iter().map(|plan| {
            let docs = NameProvider::docs(&plan.member.docs);
            let ident = NameProvider::pascal(Symbol::Declared(&plan.member.name));

            quote! { #docs #ident }
        });
        let idents: Vec<_> = members
            .iter()
            .map(|plan| NameProvider::pascal(Symbol::Declared(&plan.member.name)))
            .collect();
        let values: Vec<u32> = members.iter().map(|plan| plan.value).collect();
        let guard = members.iter().any(|plan| plan.since > 0).then(|| {
            let since_values = members.iter().map(|plan| plan.since);

            quote! {
                let added = match self {
                    #(Self::#idents => #since_values,)*
                };
                if writer.minor() < added {
                    return ::std::result::Result::Err(#core::Error::Unavailable);
                }
            }
        });
        let emit_validation = Self::emit_validation(core, &quote!(#ident), None, &quote!());
        let builder = self.tags();
        let enumeration_tags = builder.tags(tags);
        let member_values = members.iter().map(|plan| {
            let ident = NameProvider::pascal(Symbol::Declared(&plan.member.name));
            let name = &plan.member.name;
            let value = plan.value;
            let tags = builder.tags(&plan.member.tags);

            quote! {
                Self::#ident => #core::ValueRef::Member { name: #name, value: #value, tags: #tags },
            }
        });

        quote! {
            #docs
            #derives
            #[repr(u32)]
            pub enum #ident {
                #(#variants = #values,)*
            }

            impl ::std::convert::From<#ident> for u32 {
                fn from(value: #ident) -> Self {
                    value as u32
                }
            }

            impl ::std::convert::TryFrom<u32> for #ident {
                type Error = u32;

                fn try_from(value: u32) -> ::std::result::Result<Self, u32> {
                    ::std::result::Result::Ok(match value {
                        #(#values => Self::#idents,)*
                        value => return ::std::result::Result::Err(value),
                    })
                }
            }

            impl #core::BinaryEncode for #ident {
                fn write(&self, writer: &mut #core::BinaryWriter) -> ::std::result::Result<(), #core::Error> {
                    #guard
                    writer.u32(u32::from(*self));
                    ::std::result::Result::Ok(())
                }
            }

            impl #core::BinaryDecode for #ident {
                fn read(reader: &mut #core::BinaryReader<'_>) -> ::std::result::Result<Self, #core::Error> {
                    let value = reader.u32()?;
                    Self::try_from(value).map_err(|value| #core::Error::Decode(#core::DecodeFailure::UnknownMember { value, name: #name }))
                }
            }

            #emit_validation

            impl #core::Metadata for #ident {
                const TAGS: &'static [#core::Tag] = #enumeration_tags;
            }

            impl #core::AsValueRef for #ident {
                fn as_value_ref(&self) -> #core::ValueRef<'_> {
                    match *self {
                        #(#member_values)*
                    }
                }
            }
        }
    }

    fn bindings(fields: &FieldsPlan<'_>) -> TokenStream2 {
        let fields = fields.fields().map(|field| {
            let ident = NameProvider::field(field.field);
            let binding = FieldsCodec::binding(field.field);

            quote! { #ident: #binding }
        });

        quote! { #(#fields),* }
    }

    fn version_guard(core: &TokenStream2, minor: u16) -> Option<TokenStream2> {
        (minor > 0).then(|| {
            quote! {
                if writer.minor() < #minor {
                    return ::std::result::Result::Err(#core::Error::Unavailable);
                }
            }
        })
    }

    fn derives(copy: bool, eq: bool) -> TokenStream2 {
        let copy = copy.then(|| quote!(, ::std::marker::Copy));
        let eq = eq.then(|| quote!(, ::std::cmp::Eq, ::std::hash::Hash));

        quote! {
            #[derive(::std::fmt::Debug, ::std::clone::Clone, ::std::cmp::PartialEq #copy #eq)]
        }
    }
}
