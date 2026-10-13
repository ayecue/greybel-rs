use gen_lsp_types::Request as LspRequest;
use serde::Deserialize;
use serde_json::Value;

use super::RequestId;

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    pub params: Value,
}

impl Request {
    #[must_use]
    pub fn new<R: LspRequest>(id: RequestId, params: R::Params) -> Self {
        Self {
            id,
            method: R::METHOD.as_str().to_owned(),
            params: serde_json::to_value(params).expect("request parameters serialize"),
        }
    }

    #[must_use]
    pub fn is<R: LspRequest>(&self) -> bool {
        self.method == R::METHOD.as_str()
    }

    pub fn params<R: LspRequest>(&self) -> serde_json::Result<R::Params> {
        R::Params::deserialize(&self.params)
    }
}
