use gen_lsp_types::Notification as LspNotification;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

impl Notification {
    #[must_use]
    pub fn new<N: LspNotification>(params: N::Params) -> Self {
        Self {
            method: N::METHOD.as_str().to_owned(),
            params: serde_json::to_value(params).expect("notification parameters serialize"),
        }
    }

    #[must_use]
    pub fn is<N: LspNotification>(&self) -> bool {
        self.method == N::METHOD.as_str()
    }

    pub fn params<N: LspNotification>(&self) -> serde_json::Result<N::Params> {
        N::Params::deserialize(&self.params)
    }
}
