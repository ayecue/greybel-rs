use syn::{Attribute, Fields, Ident};

const ATTR_CHILD: &str = "child";
const ATTR_BODY: &str = "body";
const ATTR_SCOPE: &str = "scope";

pub struct FieldTagState<'a> {
    pub children: Vec<&'a Ident>,
    pub body: Option<&'a Ident>,
    pub scope: Option<&'a Ident>,
}

impl<'a> FieldTagState<'a> {
    pub fn new(
        children: Vec<&'a Ident>,
        body: Option<&'a Ident>,
        scope: Option<&'a Ident>,
    ) -> Self {
        Self {
            children,
            body,
            scope,
        }
    }

    pub fn from_named_fields(
        fields: impl Iterator<Item = (&'a Ident, &'a [Attribute])>,
    ) -> Result<Self, syn::Error> {
        let mut children = Vec::new();
        let mut body = None;
        let mut scope = None;

        for (name, attrs) in fields {
            for attr in attrs {
                let slot = if attr.path().is_ident(ATTR_CHILD) {
                    attr.meta.require_path_only()?;
                    children.push(name);
                    continue;
                } else if attr.path().is_ident(ATTR_BODY) {
                    &mut body
                } else if attr.path().is_ident(ATTR_SCOPE) {
                    &mut scope
                } else {
                    continue;
                };

                attr.meta.require_path_only()?;
                if slot.replace(name).is_some() {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "only one field per node can have this attribute",
                    ));
                }
            }
        }

        Ok(Self::new(children, body, scope))
    }

    pub fn from_fields(fields: &'a Fields) -> Result<Self, syn::Error> {
        match fields {
            Fields::Named(fields) => {
                Self::from_named_fields(fields.named.iter().filter_map(|field| {
                    field
                        .ident
                        .as_ref()
                        .map(|name| (name, field.attrs.as_slice()))
                }))
            }
            Fields::Unit => Ok(Self {
                children: Vec::new(),
                body: None,
                scope: None,
            }),
            Fields::Unnamed(fields) => Err(syn::Error::new_spanned(
                fields,
                "NodeFieldContainer needs named fields or a unit struct",
            )),
        }
    }
}
