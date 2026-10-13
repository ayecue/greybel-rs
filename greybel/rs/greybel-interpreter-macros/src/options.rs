use syn::LitStr;
use syn::meta::ParseNestedMeta;

const OPTION_LABEL: &str = "label";
const OPTION_METHOD: &str = "method";

#[derive(Default)]
pub struct Options {
    pub label: Option<LitStr>,
    pub method: bool,
}

impl Options {
    pub fn parse(&mut self, meta: &ParseNestedMeta<'_>) -> syn::Result<()> {
        if meta.path.is_ident(OPTION_LABEL) {
            self.label = Some(meta.value()?.parse()?);

            Ok(())
        } else if meta.path.is_ident(OPTION_METHOD) {
            self.method = true;

            Ok(())
        } else {
            Err(meta.error("expected `label = \"…\"` or `method`"))
        }
    }
}
