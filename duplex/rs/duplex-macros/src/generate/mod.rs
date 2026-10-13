mod codec;
mod declaration;
mod migration;
mod names;
mod protocol;
mod rust_type;
mod tags;

use std::path::PathBuf;

use declaration::DeclarationBuilder;
use duplex_protocol::{MigrationPlan, MinorVersionTable, Plan, Role, Status};
use migration::{MigrationBuilder, Side};
use proc_macro2::TokenStream as TokenStream2;
use protocol::ProtocolBuilder;
use quote::quote;
use rust_type::RustTypeMapper;
use syn::Path;

pub struct Generator<'a> {
    files: &'a [PathBuf],
    major: MinorVersionTable<'a>,
    core: TokenStream2,
}

impl<'a> Generator<'a> {
    pub fn new(files: &'a [PathBuf], major: MinorVersionTable<'a>) -> Self {
        Self {
            files,
            major,
            core: quote!(::duplex_core),
        }
    }

    pub fn with_core(mut self, core: Option<&Path>) -> Self {
        if let Some(core) = core {
            self.core = quote!(#core);
        }

        self
    }

    pub fn types(&self) -> TokenStream2 {
        let plan = Plan::of(self.major);
        let types = RustTypeMapper::new(&self.core, &plan, None);
        let declarations = self.declarations(&plan, &types);
        let patterns = types.patterns();
        let tracked_files = self.tracked_files();

        quote! {
            #tracked_files
            #patterns
            #declarations
        }
    }

    pub fn protocol(&self, role: Role, types: Option<&Path>) -> syn::Result<TokenStream2> {
        let plan = Plan::of(self.major);
        let latest = self.major.latest();
        let Some(protocol) = &plan.protocol else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("{} declares no protocol", latest.version),
            ));
        };

        if *latest.definition.status() == Status::Unavailable {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("{} is unsupported, which no peer speaks", latest.version),
            ));
        }

        let rust_types = RustTypeMapper::new(&self.core, &plan, types);
        let declarations = types
            .is_none()
            .then(|| self.declarations(&plan, &rust_types));
        let protocol =
            ProtocolBuilder::new(&self.core, protocol.role(role), &rust_types).generate();
        let patterns = rust_types.patterns();
        let tracked_files = self.tracked_files();

        Ok(quote! {
            #tracked_files
            #patterns
            #declarations
            #protocol
        })
    }

    pub fn migration(
        &self,
        older: MinorVersionTable<'_>,
        older_path: &Path,
        newer_path: &Path,
    ) -> syn::Result<TokenStream2> {
        let newer_plan = Plan::of(self.major);
        let older_plan = Plan::of(older);
        let (Some(older), Some(newer)) = (
            Side::new(&self.core, &older_plan, older_path),
            Side::new(&self.core, &newer_plan, newer_path),
        ) else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "both majors declare a protocol",
            ));
        };
        let plan = MigrationPlan::of(&older_plan, &newer_plan)
            .map_err(|error| syn::Error::new(proc_macro2::Span::call_site(), error))?;
        let migration = MigrationBuilder::new(&self.core, plan, older, newer).generate();
        let tracked_files = self.tracked_files();

        Ok(quote! {
            #tracked_files
            #migration
        })
    }

    fn declarations<'p>(
        &'p self,
        plan: &'p Plan<'p>,
        types: &'p RustTypeMapper<'p>,
    ) -> TokenStream2 {
        let builder = DeclarationBuilder::new(&self.core, types);
        let declarations = plan.types.iter().map(|plan| builder.declaration(plan));
        let constants = plan.constants.iter().map(|plan| builder.constant(plan));

        quote! {
            #(#constants)*
            #(#declarations)*
        }
    }

    fn tracked_files(&self) -> TokenStream2 {
        let files = self.files.iter().map(|file| file.display().to_string());

        quote! { #(const _: &[u8] = ::std::include_bytes!(#files);)* }
    }
}
