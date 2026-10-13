use gen_lsp_types::Request as LspRequest;
use serde::Deserialize;
use serde_json::Value;

use super::RequestId;
use crate::error::{ResponseError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub id: Option<RequestId>,
    pub outcome: Result<Value>,
}

impl Response {
    #[must_use]
    pub fn ok<R: LspRequest>(id: RequestId, result: R::Result) -> Self {
        Self {
            id: Some(id),
            outcome: Ok(serde_json::to_value(result).expect("results serialize")),
        }
    }

    #[must_use]
    pub fn error(id: Option<RequestId>, error: ResponseError) -> Self {
        Self {
            id,
            outcome: Err(error),
        }
    }

    #[must_use]
    pub fn result<R: LspRequest>(&self) -> Option<R::Result> {
        let value = self.outcome.as_ref().ok()?;

        R::Result::deserialize(value).ok()
    }
}
